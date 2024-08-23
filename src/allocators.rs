use axum::{extract::{Query, State}, response::Json};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::types::{Histogram, HistogramEntry, JsonResult};

#[derive(Serialize)]
pub struct AllocatorsRetrievability {
    avg_score: f64,
    allocators_retrievability_score_histogram: Histogram,
}

#[derive(Serialize)]
pub struct AllocatorsBiggestClientDistribution {
    allocators_biggest_client_distribution_histogram: Histogram,
}

#[derive(Deserialize, Serialize)]
pub struct AllocatorsSpsComplianceParameters {
    min_compliance_score: i32,
    max_compliance_score: i32,
}

#[derive(Serialize)]
pub struct AllocatorsSpsCompliance {
    parameters: AllocatorsSpsComplianceParameters,
    allocators_sps_compliance_distribution_histogram: Histogram,
}


pub async fn allocators_retrievability(
    State(pool): State<PgPool>,
) -> JsonResult<AllocatorsRetrievability> {
    let summary = sqlx::query!(
        r#"
            with allocator_retrievability as (
                select
                    allocator,
                    sum(total_deal_size*coalesce(success_rate, 0))/sum(total_deal_size) as score
                from allocator_distribution
                inner join provider_distribution using (client)
                left join provider_retrievability using (provider)
                group by allocator
            )
            select
                100*avg(score) as "avg_score!",
                count(*) as "total_count!"
            from allocator_retrievability
    "#
    )
    .fetch_one(&pool)
    .await?;

    let histogram = sqlx::query_as!(
        HistogramEntry,
        r#"
            with allocator_retrievability as (
                select
                    allocator,
                    sum(total_deal_size*coalesce(success_rate, 0))/sum(total_deal_size) as score
                from provider_distribution
                inner join allocator_distribution using (client)
                left join provider_retrievability using (provider)
                group by allocator
            )
            select
                ceil(score*20)*5 - 5 as "value_from_exclusive!",
                ceil(score*20)*5 as "value_to_inclusive!",
                count(*) as "count!"
            from allocator_retrievability
            group by 1, 2
            order by 1;
        "#
    )
    .fetch_all(&pool)
    .await?;

    Ok(Json(AllocatorsRetrievability {
        avg_score: summary.avg_score,
        allocators_retrievability_score_histogram: Histogram {
            total_count: summary.total_count,
            buckets: histogram,
        },
    }))
}

pub async fn allocators_biggest_client_distribution(
    State(pool): State<PgPool>,
) -> JsonResult<AllocatorsBiggestClientDistribution> {
    let summary = sqlx::query!(
        r#"
        select count(distinct allocator) as "total_count!"
        from allocator_distribution
    "#
    )
    .fetch_one(&pool)
    .await?;

    let histogram = sqlx::query_as!(
        HistogramEntry,
        r#"
            with allocators_with_ratio as (
                select
                    allocator,
                    max(sum_of_allocations)/sum(sum_of_allocations) biggest_to_total_ratio
                from allocator_distribution
                group by 1
            )
            select
                100*ceil(biggest_to_total_ratio::float8*20)/20 - 5 as "value_from_exclusive!",
                100*ceil(biggest_to_total_ratio::float8*20)/20 as "value_to_inclusive!",
                count(*) as "count!"
            from allocators_with_ratio
            group by 1, 2
            order by 1;
        "#
    )
    .fetch_all(&pool)
    .await?;

    Ok(Json(AllocatorsBiggestClientDistribution {
        allocators_biggest_client_distribution_histogram: Histogram {
            total_count: summary.total_count,
            buckets: histogram,
        },
    }))
}

pub async fn allocators_sps_compliance(
    Query(params): Query<AllocatorsSpsComplianceParameters>,
    State(pool): State<PgPool>,
) -> JsonResult<AllocatorsSpsCompliance> {
    let summary = sqlx::query!(
        r#"
            select
                count(distinct allocator) as "total_count!"
            from allocator_distribution
            inner join provider_distribution using (client)
    "#
    )
    .fetch_one(&pool)
    .await?;

    let histogram = sqlx::query_as!(
        HistogramEntry,
        r#"
            with
                average_retrievability as (
                    select avg(success_rate) as avg_retrievability_score from provider_retrievability
                ),
                client_info as (
                    select
                        provider,
                        count(*) as client_count,
                        max(total_deal_size)/sum(total_deal_size) as biggest_deal_ratio
                    from provider_distribution
                    group by provider
                ),
                provider_compliance as (
                    select
                        provider,
                        success_rate > avg_retrievability_score as above_avg_retrievability_score, -- FIXME inconsistent with sp graph?
                        client_count > 3 as more_than_3_clients,
                        biggest_deal_ratio <= 30 as biggest_deal_max_30_pct
                    from
                        average_retrievability,
                        provider_retrievability
                    inner join client_info using (provider)
                ),
                provider_scores as (
                    select
                        provider,
                        above_avg_retrievability_score::int + more_than_3_clients::int + biggest_deal_max_30_pct::int as compliance_score
                    from provider_compliance
                ),
                allocators_with_providers as (
                    select distinct
                        allocator,
                        provider
                    from allocator_distribution
                    inner join provider_distribution using (client)
                ),
                allocator_info as (
                    select
                        allocator,
                        count(*) as num_of_providers
                    from allocators_with_providers
                    group by 1
                ),
                allocators_with_counted_sp_scores as (
                    select
                        allocator,
                        count(*) num_of_providers_with_score
                    from allocators_with_providers
                    left join provider_scores using (provider)
                    where coalesce(compliance_score, 0) between $1 and $2
                    group by 1
                )
            select
                100*ceil(coalesce(num_of_providers_with_score, 0)::float8/num_of_providers*20)/20 - 5 as "value_from_exclusive!",
                100*ceil(coalesce(num_of_providers_with_score, 0)::float8/num_of_providers*20)/20 as "value_to_inclusive!",
                count(*) as "count!"
            from allocator_info
            left join allocators_with_counted_sp_scores using (allocator)
            group by 1, 2
            order by 1;
        "#,
        params.min_compliance_score,
        params.max_compliance_score,
    )
    .fetch_all(&pool)
    .await?;

    Ok(Json(AllocatorsSpsCompliance {
        parameters: params,
        allocators_sps_compliance_distribution_histogram: Histogram {
            total_count: summary.total_count,
            buckets: histogram,
        },
    }))
}