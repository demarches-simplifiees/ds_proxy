mod lib;

fn main() -> std::io::Result<()> {
    Ok(lib::send_to_swift::send())
}

