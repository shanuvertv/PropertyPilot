// Make cargo rebuild the crate whenever a migration file is added or edited, so
// `sqlx::migrate!()` always embeds the current directory.
fn main() {
    println!("cargo:rerun-if-changed=migrations");
}
