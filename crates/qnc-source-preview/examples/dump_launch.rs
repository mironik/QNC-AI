//! Prints, as JSON, what the Broadcast Player's bootstrap carries for one clip of the active
//! project (`input`, `media_binding`, `sound_binding`), exactly as the preview would launch
//! it. Read-only: nothing is written anywhere. Argument: a clip name (or none: the first
//! imported clip). For the live proof of another client of the same player program.

use qnc_content_read::ContentReader;
use qnc_player_client::LaunchInput;
use qnc_player_input::InputReader;
use qnc_player_launcher::SourceTransportBinding;
use qnc_work_settings::SettingsReader;

fn main() -> Result<(), String> {
    let wanted = std::env::args().nth(1);
    let root = qnc_dev_diagnostics::resolve_qnc_root().ok_or("QNC root not found")?;
    let reader = SettingsReader::from_root(&root).map_err(|e| e.to_string())?;
    let settings = reader.read().map_err(|e| e.to_string())?;
    let content = ContentReader::for_project(&reader, &settings)?;
    let clips = content.summaries()?;
    let clip = clips
        .iter()
        .find(|c| wanted.as_deref().map_or(c.imported, |name| c.name == name))
        .ok_or("no such clip (or none imported)")?;
    let player_content = std::sync::Arc::new(qnc_player_clip_content::PlayerClipContent {
        target: qnc_content_store::ContentTarget::for_project(&reader, &settings).map_err(|e| e.to_string()),
    });
    let input = InputReader::with_content_reader(reader.clone(), player_content)
        .load(&settings.workspace_db_uri, &clip.clip_id)
        .map_err(|e| e.to_string())?;
    let bindings = qnc_source_bindings::load(&root)?;
    let mut sources: Vec<SourceTransportBinding> = qnc_source_bindings::with_mounted(&bindings.sources)
        .iter()
        .filter_map(|b| b.file.as_ref().map(|root| SourceTransportBinding::local(b.uri.clone(), root.clone())))
        .collect();
    if let (Ok(Some(dir)), Ok(uri)) = (reader.local_workspace_dir(&settings), settings.project_media_source_uri()) {
        sources.push(SourceTransportBinding::local(uri, dir));
    }
    let launch = qnc_player_launcher::prepare_launch(input, &sources, std::path::PathBuf::from("unused"))?;
    let LaunchInput::Clip { input, media_binding, sound_binding, .. } = launch.input else {
        return Err("a clip".into());
    };
    let played = serde_json::json!({ "input": input, "media_binding": media_binding, "sound_binding": sound_binding });
    println!("{}", serde_json::to_string(&played).map_err(|e| e.to_string())?);
    eprintln!("clip {} ({}), {} frames", clip.name, clip.clip_id, clip.duration_frames);
    Ok(())
}
