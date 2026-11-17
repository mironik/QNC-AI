//! Lists the Ingest bases, or deletes the one whose label holds the text given (the same delete as
//! the administrator's form: asked of the owner, closed, forgotten). `delete_base ROOT [TEXT]`.

fn main() {
    let mut args = std::env::args().skip(1);
    let root = std::path::PathBuf::from(args.next().expect("the QNC root"));
    let rows = qnc_base_admin::read(&root).expect("bases read");
    let Some(text) = args.next() else {
        for row in rows {
            println!("{} | {} | may delete: {:?}", row.base.label(), row.base.id, row.may_delete.is_ok());
        }
        return;
    };
    let found: Vec<_> = rows.into_iter().filter(|row| row.base.label().contains(&text)).collect();
    let [row] = found.as_slice() else { panic!("{} bases hold {text}", found.len()) };
    let delete = qnc_base_admin::Delete::start(&root, &row.base);
    loop {
        if let Some(result) = delete.poll() {
            println!("{}: {result:?}", row.base.label());
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(200));
    }
}
