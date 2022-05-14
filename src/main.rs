use revproc1::utils::memory;

#[tokio::main]
async fn main() -> Result<(), ()> {
    let some_memory = memory::TreeMemory::new();
    println!("Hello, world!");
    Ok(())
}
