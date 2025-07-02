# Remitano API Client

## Usage

```toml
[dependencies]
remitano-api = "0.0.2"

```

```rust
#[derive(Deserialize, Debug)]
pub struct User {
    pub id: u64,
    pub username: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let remitano_api = RemitanoApiBuilder::default()
        .key(dotenv!("REMITANO_API_KEY").to_string())
        .secret(dotenv!("REMITANO_API_SECRET").to_string())
        .build()?;

    let user: User = remitano_api
        .request(Method::GET, "users/me", None, None)
        .await?;

    println!("user: {:?}", &user);

    Ok(())
}
```

