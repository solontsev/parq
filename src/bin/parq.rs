use anyhow::Result;
use clap::Parser;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use crossterm::{
    event,
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
};
use parq::{ParquetFileData, app::App, args::Args, source};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use std::io;

fn main() {
    match run() {
        Ok(_) => (),
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    }
}

fn run() -> Result<()> {
    let args = Args::parse();
    let file_source = source::FileSource::parse(&args.filename)?;
    let source_file = file_source.load()?;
    let file_info = ParquetFileData::new(&source_file)?;

    let mut terminal = init_terminal()?;
    let mut app = App::new(file_info, args);
    app.run(&mut terminal)?;
    restore_terminal(&mut terminal)?;

    Ok(())
}

fn init_terminal() -> Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    // disable mouse capture first
    let _ = execute!(terminal.backend_mut(), DisableMouseCapture);

    // small delay to let mouse events settle
    std::thread::sleep(std::time::Duration::from_millis(10));

    // clear the event queue
    while event::poll(std::time::Duration::from_millis(0)).unwrap_or(false) {
        let _ = event::read();
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}
