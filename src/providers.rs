use axum::{extract::State, response::Json};
use serde::Serialize;
use sqlx::PgPool;

use crate::types::{Histogram, HistogramEntry, JsonResult};

#[derive(Serialize)]
pub struct ProvidersRetrievabilityScore {
    avg_success_rate_pct: f64,
    providers_retrievability_score_histogram: Histogram,
}

#[derive(Serialize)]
pub struct ProvidersClients {
    providers_client_count_histogram: Histogram,
}

#[derive(Serialize)]
pub struct ProvidersBiggestClientDistribution {
    providers_biggest_client_distribution_histogram: Histogram,
}

pub async fn providers_retrievability(
    State(pool): State<PgPool>,
) -> JsonResult<ProvidersRetrievabilityScore> {
    let summary = sqlx::query!(
        r#"
        select
            100*avg(coalesce(success_rate, 0)) as "avg_success_rate_pct!",
            count(*) as "total_count!"
        from providers
        left join provider_retrievability using (provider)
    "#
    )
    .fetch_one(&pool)
    .await?;

    let histogram = sqlx::query_as!(
        HistogramEntry,
        r#"
            select 
                100*ceil(coalesce(success_rate, 0)*20)/20 - 5 as "value_from_exclusive!",
                100*ceil(coalesce(success_rate, 0)*20)/20 as "value_to_inclusive!",
                count(*) as "count!"
            from providers
            left join provider_retrievability using (provider)
            group by 1, 2
            order by 1;
        "#
    )
    .fetch_all(&pool)
    .await?;

    Ok(Json(ProvidersRetrievabilityScore {
        avg_success_rate_pct: summary.avg_success_rate_pct,
        providers_retrievability_score_histogram: Histogram {
            total_count: summary.total_count,
            buckets: histogram,
        },
    }))
}

pub async fn providers_clients(State(pool): State<PgPool>) -> JsonResult<ProvidersClients> {
    let summary = sqlx::query!(
        r#"
        select count(distinct provider) as "total_count!"
        from provider_distribution
    "#
    )
    .fetch_one(&pool)
    .await?;

    let histogram = sqlx::query_as!(
        HistogramEntry,
        r#"
            with clients_per_provider as (
                select
                    count(distinct client) as clients_count
                    from provider_distribution
                    group by provider
            )
            select
                (clients_count - 1)::float as "value_from_exclusive!",
                clients_count::float as "value_to_inclusive!",
                count(*) as "count!"
            from clients_per_provider
            group by 1, 2
            order by 1;
        "#
    )
    .fetch_all(&pool)
    .await?;

    Ok(Json(ProvidersClients {
        providers_client_count_histogram: Histogram {
            total_count: summary.total_count,
            buckets: histogram,
        },
    }))
}

pub async fn providers_biggest_client_distribution(
    State(pool): State<PgPool>,
) -> JsonResult<ProvidersBiggestClientDistribution> {
    let summary = sqlx::query!(
        r#"
        select count(distinct provider) as "total_count!"
        from provider_distribution
    "#
    )
    .fetch_one(&pool)
    .await?;

    let histogram = sqlx::query_as!(
        HistogramEntry,
        r#"
            with providers_with_ratio as (
                select
                    provider,
                    max(total_deal_size)/sum(total_deal_size) biggest_to_total_ratio
                from provider_distribution
                group by 1
            )
            select
                100*ceil(biggest_to_total_ratio::float8*20)/20 - 5 as "value_from_exclusive!",
                100*ceil(biggest_to_total_ratio::float8*20)/20 as "value_to_inclusive!",
                count(*) as "count!"
            from providers_with_ratio
            group by 1, 2
            order by 1;
        "#
    )
    .fetch_all(&pool)
    .await?;

    Ok(Json(ProvidersBiggestClientDistribution {
        providers_biggest_client_distribution_histogram: Histogram {
            total_count: summary.total_count,
            buckets: histogram,
        },
    }))
}
