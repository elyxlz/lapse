use crate::db::{Card as DbCard, UpdatedCard};
use anyhow::Result;
use chrono::{DateTime, TimeZone, Utc};
use rs_fsrs::{Card as FsrsCard, Rating, State, FSRS};

pub struct ScheduleResult {
    pub updated: UpdatedCard,
    pub elapsed_days: f64,
    pub scheduled_days: f64,
    pub state_before: i64,
}

pub fn rating_from_u8(r: u8) -> Result<Rating> {
    Ok(match r {
        1 => Rating::Again,
        2 => Rating::Hard,
        3 => Rating::Good,
        4 => Rating::Easy,
        _ => anyhow::bail!("invalid rating {} (expected 1..=4)", r),
    })
}

fn state_from_i64(s: i64) -> State {
    match s {
        1 => State::Learning,
        2 => State::Review,
        3 => State::Relearning,
        _ => State::New,
    }
}

fn state_to_i64(s: State) -> i64 {
    s as i64
}

fn ms_to_dt(ms: i64) -> DateTime<Utc> {
    Utc.timestamp_millis_opt(ms).single().unwrap_or_else(Utc::now)
}

fn db_card_to_fsrs(card: &DbCard, now: DateTime<Utc>) -> FsrsCard {
    let last_review = card.last_review.map(ms_to_dt).unwrap_or(now);
    let due = if card.state == 0 { now } else { ms_to_dt(card.due) };
    FsrsCard {
        due,
        stability: card.stability,
        difficulty: card.difficulty,
        elapsed_days: 0,    // recomputed by Scheduler::new
        scheduled_days: 0,  // recomputed by Scheduler::new
        reps: card.reps as i32,
        lapses: card.lapses as i32,
        state: state_from_i64(card.state),
        last_review,
    }
}

pub fn schedule(card: &DbCard, rating_u8: u8, now: DateTime<Utc>) -> Result<ScheduleResult> {
    let rating = rating_from_u8(rating_u8)?;
    let state_before = card.state;
    let fsrs_card = db_card_to_fsrs(card, now);

    let fsrs = FSRS::default();
    let info = fsrs.next(fsrs_card, now, rating);
    let scheduled = info.card;
    let log = info.review_log;

    Ok(ScheduleResult {
        updated: UpdatedCard {
            state: state_to_i64(scheduled.state),
            due: scheduled.due.timestamp_millis(),
            stability: scheduled.stability,
            difficulty: scheduled.difficulty,
            reps: scheduled.reps as i64,
            lapses: scheduled.lapses as i64,
            last_review: now.timestamp_millis(),
        },
        elapsed_days: log.elapsed_days as f64,
        scheduled_days: log.scheduled_days as f64,
        state_before,
    })
}
