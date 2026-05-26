use crate::db::{Card as DbCard, UpdatedCard};
use anyhow::{anyhow, Result};
use chrono::{DateTime, TimeZone, Utc};
use rs_fsrs::{Card as FsrsCard, Rating, State, FSRS};

/// Anki-default learning steps: 1 minute, then 10 minutes. A card answered
/// Easy from the final step graduates to the FSRS-managed Review state.
/// Hard at any step resets the card back to step 0.
const LEARN_STEPS_MS: &[i64] = &[60_000, 600_000];

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

fn ms_to_dt(ms: i64) -> Result<DateTime<Utc>> {
    Utc.timestamp_millis_opt(ms)
        .single()
        .ok_or_else(|| anyhow!("timestamp out of range: {ms}"))
}

fn db_card_to_fsrs(card: &DbCard, now: DateTime<Utc>) -> Result<FsrsCard> {
    let last_review = match card.last_review {
        Some(ms) => ms_to_dt(ms)?,
        None => now,
    };
    let due = if card.state == 0 { now } else { ms_to_dt(card.due)? };
    Ok(FsrsCard {
        due,
        stability: card.stability,
        difficulty: card.difficulty,
        elapsed_days: 0,
        scheduled_days: 0,
        reps: card.reps as i32,
        lapses: card.lapses as i32,
        state: state_from_i64(card.state),
        last_review,
    })
}

pub fn schedule(card: &DbCard, rating_u8: u8, now: DateTime<Utc>) -> Result<ScheduleResult> {
    let rating = rating_from_u8(rating_u8)?;
    let state_before = card.state;
    let now_ms = now.timestamp_millis();

    // Always run FSRS so we have a stability/difficulty trajectory ready
    // for when the card eventually graduates. We may override its state +
    // due below for the short-term learning ladder.
    let fsrs_card = db_card_to_fsrs(card, now)?;
    let fsrs = FSRS::default();
    let info = fsrs.next(fsrs_card, now, rating);
    let scheduled = info.card;
    let log = info.review_log;

    let mut updated = UpdatedCard {
        state: state_to_i64(scheduled.state),
        due: scheduled.due.timestamp_millis(),
        stability: scheduled.stability,
        difficulty: scheduled.difficulty,
        reps: scheduled.reps as i64,
        lapses: scheduled.lapses as i64,
        last_review: now_ms,
        learn_step: card.learn_step,
    };

    // Cards in New or Learning state walk the Anki-style learning ladder.
    // Cards already in Review/Relearning are FSRS-managed; clear learn_step.
    let in_short_term = state_before == 0 || state_before == 1;
    if !in_short_term {
        updated.learn_step = None;
    } else {
        match rating {
            Rating::Again | Rating::Hard => {
                // Reset to the bottom of the ladder.
                updated.state = State::Learning as i64;
                updated.due = now_ms + LEARN_STEPS_MS[0];
                updated.learn_step = Some(0);
            }
            Rating::Good | Rating::Easy => {
                let current_step = if state_before == 0 {
                    // Brand-new card answered correctly — start at step 0
                    // BEFORE advancing, so the next index is step 1.
                    -1
                } else {
                    card.learn_step.unwrap_or(0)
                };
                let next_step = current_step + 1;
                if (next_step as usize) >= LEARN_STEPS_MS.len() {
                    // Graduate. Keep FSRS's Review state + due that we
                    // already populated into `updated` above.
                    updated.state = State::Review as i64;
                    updated.learn_step = None;
                    if updated.due <= now_ms {
                        // FSRS sometimes returns a same-day due for fresh
                        // graduations; ensure at least one day out.
                        updated.due = now_ms + 24 * 60 * 60 * 1000;
                    }
                } else {
                    updated.state = State::Learning as i64;
                    updated.due = now_ms + LEARN_STEPS_MS[next_step as usize];
                    updated.learn_step = Some(next_step);
                }
            }
        }
    }

    Ok(ScheduleResult {
        updated,
        elapsed_days: log.elapsed_days as f64,
        scheduled_days: log.scheduled_days as f64,
        state_before,
    })
}
