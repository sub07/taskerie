fn main() -> anyhow::Result<()> {
    let taskerie = taskerie::load("../taskerie.example.yaml")?;
    Ok(())
}
