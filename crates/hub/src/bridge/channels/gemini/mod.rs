//! C14.4: Gemini / Antigravity (`agy`) managed session channel module.
//!
//! Provides the kill -> capture -> relaunch continuation flow for managed `agy`
//! sessions when Task or Wake delivery is requested.
#![allow(dead_code, unused_imports)]

mod relaunch;

pub use relaunch::{
    is_pid_running, kill_managed_agy_process, parse_agy_resume_conversation_id,
    relaunch_and_deliver_gemini_task, relaunch_and_deliver_gemini_task_with,
    resolve_gemini_continuation_id,
};
