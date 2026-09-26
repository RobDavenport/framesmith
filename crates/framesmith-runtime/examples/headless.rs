//! Run with: cargo run --example headless -- path/to/character.fspk
use framesmith_fspack::PackView;
use framesmith_runtime::{init_resources, next_frame, CharacterState, FrameInput};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .ok_or("expected a character.fspk path")?;
    let bytes = std::fs::read(path)?;
    let pack = PackView::parse(&bytes)?;
    let states = pack
        .states()
        .ok_or("this example requires compiled character tables")?;
    if states.is_empty() {
        return Err("character has no states".into());
    }
    let mut state = CharacterState::default();
    if !init_resources(&mut state, &pack) {
        return Err("resource definitions exceed helper capacity".into());
    }
    let snapshot = state;
    for _ in 0..120 {
        state = next_frame(&state, &pack, &FrameInput::default()).state;
    }
    let mut replay = snapshot;
    for _ in 0..120 {
        replay = next_frame(&replay, &pack, &FrameInput::default()).state;
    }
    assert_eq!(state, replay);
    println!(
        "FSPK v{}: {} states; 120 deterministic frames replayed",
        pack.version(),
        states.len()
    );
    if let Some(data) = pack.state_data(0) {
        println!(
            "first state identity: {:?}; engine payload fields: {}",
            pack.state_id(0),
            data.children().count()
        );
    }
    if std::env::args().any(|arg| arg == "--bench") {
        #[cfg(feature = "std")]
        {
            let owned = framesmith_fspack::OwnedPack::new(bytes.clone())?;
            let start = std::time::Instant::now();
            for _ in 0..100_000 {
                std::hint::black_box(owned.view());
            }
            println!(
                "cached view: {:.1} ns/pack (100000 iterations)",
                start.elapsed().as_nanos() as f64 / 100_000.0
            );
        }
        let start = std::time::Instant::now();
        for _ in 0..10_000 {
            std::hint::black_box(PackView::parse(std::hint::black_box(&bytes))?);
        }
        println!(
            "parse: {:.1} ns/pack (10000 iterations, {} bytes)",
            start.elapsed().as_nanos() as f64 / 10_000.0,
            bytes.len()
        );
        let start = std::time::Instant::now();
        for _ in 0..100_000 {
            state = next_frame(std::hint::black_box(&state), &pack, &FrameInput::default()).state;
        }
        std::hint::black_box(state);
        println!(
            "step: {:.1} ns/frame (100000 iterations)",
            start.elapsed().as_nanos() as f64 / 100_000.0
        );
    }
    Ok(())
}
