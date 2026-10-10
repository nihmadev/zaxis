//! Export a colored SVG without a GUI dependency.
use std::{
    error::Error,
    io::{self, Write},
};

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let color = args.next().unwrap_or_else(|| "#2563eb".into());
    let size: u32 = args
        .next()
        .map(|value| value.parse())
        .transpose()?
        .unwrap_or(32);
    if size == 0 || args.next().is_some() {
        return Err("usage: export [color] [positive-size]".into());
    }
    writeln!(
        io::stdout().lock(),
        "{}",
        z_icons::USER.render().color(&color).size(size, size)
    )?;
    Ok(())
}
