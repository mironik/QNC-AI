//! The template choice of the user signed in and of the computer: `template_choice <QNC root>`;
//! with `--forget-mine` the user's choice is forgotten, so the computer's holds for them.
fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let root = std::path::PathBuf::from(args.first().ok_or("usage: template_choice QNC_ROOT [--forget-mine]")?);
    let store = qnc_project_store::ProjectStore::open(&root)?;
    println!("before (user, computer): {:?}", store.template_choices()?);
    if args.iter().any(|arg| arg == "--forget-mine") {
        store.forget_template_choice()?;
        println!("after (user, computer): {:?}", store.template_choices()?);
    }
    println!("used: {}", store.selected_template_settings()?.template_id);
    Ok(())
}
