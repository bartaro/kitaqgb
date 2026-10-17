//! Stable UTF-8 JSON C ABI, also used by the dependency-free Python bindings.
use crate::{
    editing::{edit_notes, NoteEdit},
    services::*,
    SequenceProject,
};
use anyhow::{ensure, Result};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{
    ffi::{c_char, CStr, CString},
    path::PathBuf,
};

#[derive(Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
enum Request {
    New {
        #[serde(default)]
        demo: bool,
    },
    Load {
        path: PathBuf,
    },
    Validate {
        project: SequenceProject,
    },
    Save {
        project: SequenceProject,
        path: PathBuf,
    },
    AddNote {
        project: SequenceProject,
        track: usize,
        start: u32,
        length: u32,
        pitch: u8,
        velocity: u8,
    },
    Edit {
        project: SequenceProject,
        ids: Vec<u64>,
        action: String,
        #[serde(default)]
        value: i64,
    },
    RenderWav {
        project: SequenceProject,
        path: PathBuf,
        player: Option<usize>,
    },
    ExportKitaqgb {
        project: SequenceProject,
        path: PathBuf,
        #[serde(default)]
        player: usize,
    },
    ExportLink16 {
        project: SequenceProject,
        path: PathBuf,
    },
}

fn dispatch(request: Request) -> Result<Value> {
    match request {
        Request::New { demo } => Ok(serde_json::to_value(if demo {
            SequenceProject::demo()
        } else {
            SequenceProject::empty()
        })?),
        Request::Load { path } => Ok(serde_json::to_value(load_sequence(&path)?)?),
        Request::Validate { project } => {
            project.validate()?;
            Ok(json!({"summary":inspect_sequence(&project)?}))
        }
        Request::Save { project, path } => {
            save_sequence(&project, &path)?;
            Ok(json!({"path":path}))
        }
        Request::AddNote {
            mut project,
            track,
            start,
            length,
            pitch,
            velocity,
        } => {
            project.validate()?;
            ensure!(
                track < project.link16_mode.track_count(),
                "track is outside active link mode"
            );
            ensure!(
                pitch <= 47 && velocity <= 15,
                "pitch must be 0..47, velocity 0..15"
            );
            let id = project.insert_note_exact(track, start, length, pitch, velocity)?;
            project.validate()?;
            Ok(json!({"project":project,"id":id}))
        }
        Request::Edit {
            mut project,
            ids,
            action,
            value,
        } => {
            project.validate()?;
            let edit = match action.as_str() {
                "move" => NoteEdit::Move(value.try_into()?),
                "transpose" => NoteEdit::Transpose(value.try_into()?),
                "velocity" => NoteEdit::Velocity(value.try_into()?),
                "quantize" => NoteEdit::Quantize(value.try_into()?),
                "delete" => NoteEdit::Delete,
                "duplicate" => NoteEdit::Duplicate,
                _ => anyhow::bail!("unknown edit action: {action}"),
            };
            let ids = edit_notes(&mut project, &ids, edit)?;
            Ok(json!({"project":project,"ids":ids}))
        }
        Request::RenderWav {
            project,
            path,
            player,
        } => {
            render_wav(&project, &path, player)?;
            Ok(json!({"path":path}))
        }
        Request::ExportKitaqgb {
            project,
            path,
            player,
        } => {
            ensure!(
                player < project.player_count(),
                "player is outside active link mode"
            );
            Ok(json!({"bytes":export_player(&project,&path,player)?}))
        }
        Request::ExportLink16 { project, path } => Ok(serde_json::to_value(
            crate::export_link16_saves(&project, &path)?,
        )?),
    }
}

pub fn call_json(request: &str) -> String {
    let result =
        std::panic::catch_unwind(|| -> Result<Value> { dispatch(serde_json::from_str(request)?) });
    let response = match result {
        Ok(Ok(value)) => json!({"ok":true,"result":value}),
        Ok(Err(error)) => json!({"ok":false,"error":format!("{error:#}")}),
        Err(_) => json!({"ok":false,"error":"internal GBSEQ panic"}),
    };
    response.to_string()
}

/// # Safety
/// `request` must be null or a live, NUL-terminated UTF-8 string for this call.
/// Free the returned pointer exactly once with `gbseq_free_json`.
#[no_mangle]
pub unsafe extern "C" fn gbseq_call_json(request: *const c_char) -> *mut c_char {
    let response = if request.is_null() {
        json!({"ok":false,"error":"null request"}).to_string()
    } else {
        match unsafe { CStr::from_ptr(request) }.to_str() {
            Ok(text) => call_json(text),
            Err(_) => json!({"ok":false,"error":"request is not UTF-8"}).to_string(),
        }
    };
    CString::new(response).expect("JSON escapes NUL").into_raw()
}

/// # Safety
/// `response` must be null or an unfreed pointer returned by `gbseq_call_json`.
#[no_mangle]
pub unsafe extern "C" fn gbseq_free_json(response: *mut c_char) {
    if !response.is_null() {
        drop(unsafe { CString::from_raw(response) });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn json_errors_do_not_unwind_into_python() {
        let bad: Value = serde_json::from_str(&call_json("not json")).unwrap();
        assert_eq!(bad["ok"], false);
        let new: Value = serde_json::from_str(&call_json(r#"{"op":"new","demo":true}"#)).unwrap();
        assert_eq!(new["ok"], true);
        let valid = call_json(&json!({"op":"validate","project":new["result"]}).to_string());
        assert_eq!(serde_json::from_str::<Value>(&valid).unwrap()["ok"], true);
    }
}
