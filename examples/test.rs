use tokio::io::AsyncWriteExt;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    match tokio::fs::read_to_string("foo.txt").await {
        Ok(data) => {
            if data == "Foobar" {
                println!("File exists and data matches expectation");
                return Ok(());
            } else {
                println!("Unexpected contents, truncating existing file");
            }
        }
        Err(err) => {
            if err.kind() == std::io::ErrorKind::NotFound {
                println!("File not found");
            } else {
                eprintln!("Unexpected error");
                Err(err)?;
            }
        }
    }

    let mut file = tokio::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open("foo.txt")
        .await?;
    println!("Writing 'Foobar' to file");
    file.write_all(b"Foobar").await?;

    Ok(())
}
