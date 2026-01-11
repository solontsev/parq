use crate::{ParquetFileInfo, SchemaNode};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, MouseEvent, MouseEventKind};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Padding, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Tabs,
    },
};
use std::io;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    Structure,
    Schema,
    Metadata,
    Stats,
}

impl ViewMode {
    pub fn as_str(&self) -> &str {
        match self {
            ViewMode::Structure => "Structure",
            ViewMode::Schema => "Schema",
            ViewMode::Metadata => "Metadata",
            ViewMode::Stats => "Stats",
        }
    }

    pub fn all() -> [ViewMode; 4] {
        [
            ViewMode::Structure,
            ViewMode::Schema,
            ViewMode::Metadata,
            ViewMode::Stats,
        ]
    }

    pub fn index(&self) -> usize {
        match self {
            ViewMode::Structure => 0,
            ViewMode::Schema => 1,
            ViewMode::Metadata => 2,
            ViewMode::Stats => 3,
        }
    }

    pub fn from_index(index: usize) -> Self {
        match index {
            0 => ViewMode::Structure,
            1 => ViewMode::Schema,
            2 => ViewMode::Metadata,
            3 => ViewMode::Stats,
            _ => ViewMode::Structure,
        }
    }
}

pub struct App {
    pub info: ParquetFileInfo,
    pub file_path: String,
    pub current_view: ViewMode,
    pub should_quit: bool,
    pub scroll_offset: usize,
    pub max_scroll: usize,
    pub expanded_nodes: Vec<usize>,
}

impl App {
    pub fn run(&mut self, terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<()> {
        while !self.should_quit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn draw(&mut self, frame: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(3)])
            .split(frame.area());

        self.render_tabs(frame, chunks[0]);
        self.render_footer(frame, chunks[1]);
    }

    fn render_footer(&self, frame: &mut Frame, area: Rect) {
        let help_text = "[Tab/←→] Switch tabs | [1-4] Direct tab | [↑/↓/j/k] Scroll | [PgUp/PgDn] Page | [Home/End] | [q/Esc] Quit";
        let title = Line::from(vec![
            Span::raw(" File: "),
            Span::styled(&self.file_path, Style::default().fg(Color::White)),
            Span::raw(" "),
        ]);
        let footer = Paragraph::new(help_text)
            .style(Style::default().fg(Color::DarkGray))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(title)
                    .padding(Padding::horizontal(1)),
            );
        frame.render_widget(footer, area);
    }

    fn render_tabs(&mut self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(0)])
            .split(area);

        let titles: Vec<_> = ViewMode::all()
            .into_iter()
            .map(|mode| mode.as_str().to_string())
            .collect();

        let selected_index = self.current_view.index();

        let tabs = Tabs::new(titles)
            .select(selected_index)
            .style(Style::default().fg(Color::DarkGray))
            .highlight_style(
                Style::default()
                    .fg(Color::White)
                    .bg(Color::Black)
                    .add_modifier(Modifier::BOLD)
                    .add_modifier(Modifier::UNDERLINED),
            )
            .divider("|")
            .padding(" ", " ");

        frame.render_widget(tabs, chunks[0]);

        match self.current_view {
            ViewMode::Structure => self.render_structure_view(frame, chunks[1]),
            ViewMode::Schema => self.render_schema_view(frame, chunks[1]),
            ViewMode::Metadata => self.render_metadata_view(frame, chunks[1]),
            ViewMode::Stats => self.render_stats_view(frame, chunks[1]),
        }
    }

    fn render_structure_view(&mut self, frame: &mut Frame, area: Rect) {
        let info = &self.info;
        let mut lines = vec![];

        lines.push(Line::from(vec![
            Span::styled("File Version: ", Style::default().fg(Color::Cyan)),
            Span::raw(format!("{}", info.meta_info.version)),
        ]));

        lines.push(Line::from(vec![
            Span::styled("Total Rows: ", Style::default().fg(Color::Cyan)),
            Span::raw(format!("{}", info.meta_info.num_rows)),
        ]));

        lines.push(Line::from(vec![
            Span::styled("Row Groups: ", Style::default().fg(Color::Cyan)),
            Span::raw(format!("{}", info.meta_info.num_row_groups)),
        ]));

        if let Some(created_by) = &info.meta_info.created_by {
            lines.push(Line::from(vec![
                Span::styled("Created By: ", Style::default().fg(Color::Cyan)),
                Span::raw(created_by.clone()),
            ]));
        }

        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(
            "Row Groups:",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )]));

        for rg in &info.row_groups_info {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("  Row Group ", Style::default().fg(Color::Green)),
                Span::raw(format!("{}", rg.index)),
            ]));
            lines.push(Line::from(vec![
                Span::raw("    Rows: "),
                Span::raw(format!("{}", rg.num_rows)),
            ]));
            lines.push(Line::from(vec![
                Span::raw("    Total Bytes: "),
                Span::raw(format!("{}", rg.total_byte_size)),
            ]));
            lines.push(Line::from(vec![
                Span::raw("    Columns: "),
                Span::raw(format!("{}", rg.columns.len())),
            ]));
        }

        render_lines_with_scrollbar(
            &mut self.scroll_offset,
            &mut self.max_scroll,
            frame,
            area,
            lines,
        );
    }

    fn render_schema_view(&mut self, frame: &mut Frame, area: Rect) {
        let mut lines = vec![];

        {
            let schema_tree = &self.info.meta_info.schema_tree;
            render_schema_node(&schema_tree, 0, &mut lines);
        }

        render_lines_with_scrollbar(
            &mut self.scroll_offset,
            &mut self.max_scroll,
            frame,
            area,
            lines,
        );
    }

    fn render_metadata_view(&mut self, frame: &mut Frame, area: Rect) {
        let info = &self.info.meta_info;
        let mut lines = vec![];

        lines.push(Line::from(vec![Span::styled(
            "File Metadata:",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )]));
        lines.push(Line::from(""));

        lines.push(Line::from(vec![
            Span::styled("Version: ", Style::default().fg(Color::Cyan)),
            Span::raw(format!("{}", info.version)),
        ]));

        lines.push(Line::from(vec![
            Span::styled("Total Rows: ", Style::default().fg(Color::Cyan)),
            Span::raw(format!("{}", info.num_rows)),
        ]));

        lines.push(Line::from(vec![
            Span::styled("Row Groups: ", Style::default().fg(Color::Cyan)),
            Span::raw(format!("{}", info.num_row_groups)),
        ]));

        if let Some(created_by) = &info.created_by {
            lines.push(Line::from(vec![
                Span::styled("Created By: ", Style::default().fg(Color::Cyan)),
                Span::raw(created_by.clone()),
            ]));
        }

        if !info.key_value_metadata.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![Span::styled(
                "Key-Value Metadata:",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )]));

            for (key, value) in &info.key_value_metadata {
                lines.push(Line::from(vec![
                    Span::styled(format!("  {}: ", key), Style::default().fg(Color::Cyan)),
                    Span::raw(value.clone()),
                ]));
            }
        }

        render_lines_with_scrollbar(
            &mut self.scroll_offset,
            &mut self.max_scroll,
            frame,
            area,
            lines,
        );
    }

    fn render_stats_view(&mut self, frame: &mut Frame, area: Rect) {
        let row_groups = &self.info.row_groups_info;
        let mut lines = vec![];

        lines.push(Line::from(vec![Span::styled(
            "Column Statistics:",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )]));
        lines.push(Line::from(""));

        for rg in row_groups {
            lines.push(Line::from(vec![
                Span::styled("Row Group ", Style::default().fg(Color::Green)),
                Span::raw(format!("{}", rg.index)),
            ]));
            lines.push(Line::from(""));

            for col in &rg.columns {
                lines.push(Line::from(vec![
                    Span::styled("  Column: ", Style::default().fg(Color::Cyan)),
                    Span::raw(&col.name),
                ]));
                lines.push(Line::from(vec![
                    Span::raw("    Type: "),
                    Span::raw(&col.column_type),
                ]));
                lines.push(Line::from(vec![
                    Span::raw("    Compression: "),
                    Span::raw(&col.compression),
                ]));
                lines.push(Line::from(vec![
                    Span::raw("    Encodings: "),
                    Span::raw(&col.encodings),
                ]));
                lines.push(Line::from(vec![
                    Span::raw("    Values: "),
                    Span::raw(format!("{}", col.num_values)),
                ]));
                lines.push(Line::from(vec![
                    Span::raw("    Compressed Size: "),
                    Span::raw(format!("{} bytes", col.total_compressed_size)),
                ]));
                lines.push(Line::from(vec![
                    Span::raw("    Uncompressed Size: "),
                    Span::raw(format!("{} bytes", col.total_uncompressed_size)),
                ]));

                if let Some(stats) = &col.statistics {
                    if let Some(min) = &stats.min {
                        lines.push(Line::from(vec![
                            Span::raw("    Min: "),
                            Span::raw(min.clone()),
                        ]));
                    }
                    if let Some(max) = &stats.max {
                        lines.push(Line::from(vec![
                            Span::raw("    Max: "),
                            Span::raw(max.clone()),
                        ]));
                    }
                    if let Some(null_count) = stats.null_count {
                        lines.push(Line::from(vec![
                            Span::raw("    Null Count: "),
                            Span::raw(format!("{}", null_count)),
                        ]));
                    }
                    if let Some(distinct_count) = stats.distinct_count {
                        lines.push(Line::from(vec![
                            Span::raw("    Distinct Count: "),
                            Span::raw(format!("{}", distinct_count)),
                        ]));
                    }
                }

                lines.push(Line::from(""));
            }
        }

        render_lines_with_scrollbar(
            &mut self.scroll_offset,
            &mut self.max_scroll,
            frame,
            area,
            lines,
        );
    }
}

// Helper function to handle common rendering logic
fn render_lines_with_scrollbar(
    scroll_offset: &mut usize,
    max_scroll: &mut usize,
    frame: &mut Frame,
    area: Rect,
    lines: Vec<Line>,
) {
    let total_lines = lines.len();

    // First, create block with all borders to calculate actual viewport
    let temp_block = Block::default()
        .borders(Borders::ALL)
        .padding(Padding::horizontal(1));
    let temp_inner = temp_block.inner(area);
    let viewport_height = temp_inner.height as usize;

    // Now determine if we need scrollbar based on actual viewport
    let max_scroll_value = total_lines.saturating_sub(viewport_height);
    let needs_scrollbar = max_scroll_value > 0;

    // Create the actual block with correct borders
    let borders = if needs_scrollbar {
        Borders::LEFT | Borders::TOP | Borders::BOTTOM
    } else {
        Borders::ALL
    };

    let block = Block::default()
        .borders(borders)
        .padding(Padding::horizontal(1));

    *max_scroll = max_scroll_value;
    *scroll_offset = (*scroll_offset).min(max_scroll_value);

    let visible_lines: Vec<_> = lines
        .into_iter()
        .skip(*scroll_offset)
        .take(viewport_height)
        .collect();

    let paragraph = Paragraph::new(visible_lines).block(block);

    frame.render_widget(paragraph, area);

    if needs_scrollbar {
        let scrollbar = Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓"))
            .track_symbol(Some("│"))
            .thumb_symbol("█");

        let mut scrollbar_state = ScrollbarState::default()
            .content_length(max_scroll_value.max(1))
            .position(*scroll_offset);

        let scrollbar_area = Rect {
            x: area.x,
            y: area.y + 1,
            width: area.width,
            height: area.height.saturating_sub(2),
        };

        frame.render_stateful_widget(scrollbar, scrollbar_area, &mut scrollbar_state);
    }
}

impl App {
    fn handle_events(&mut self) -> io::Result<()> {
        match event::read()? {
            Event::Key(key) => {
                self.handle_key_event(key);
            }
            Event::Mouse(mouse) => {
                self.handle_mouse_event(mouse);
                // Process any additional buffered mouse events
                while event::poll(std::time::Duration::from_millis(0)).unwrap_or(false) {
                    if let Ok(Event::Mouse(m)) = event::read() {
                        self.handle_mouse_event(m);
                    } else {
                        break;
                    }
                }
            }
            Event::Resize(_, _) => {
                // Terminal resize - just continue to redraw
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_key_event(&mut self, key: KeyEvent) {
        // Only handle key press events, ignore key release and repeat
        if key.kind != KeyEventKind::Press {
            return;
        }

        // Extra safety: only quit on explicit q or Esc with no modifiers
        if matches!(key.code, KeyCode::Char('q') | KeyCode::Esc) && key.modifiers.is_empty() {
            self.quit();
            return;
        }

        match key.code {
            KeyCode::Char('1') => self.set_view(ViewMode::Structure),
            KeyCode::Char('2') => self.set_view(ViewMode::Schema),
            KeyCode::Char('3') => self.set_view(ViewMode::Metadata),
            KeyCode::Char('4') => self.set_view(ViewMode::Stats),
            KeyCode::Tab | KeyCode::Right => self.next_tab(),
            KeyCode::BackTab | KeyCode::Left => self.previous_tab(),
            KeyCode::Down | KeyCode::Char('j') => self.scroll_down(),
            KeyCode::Up | KeyCode::Char('k') => self.scroll_up(),
            KeyCode::PageDown => self.page_down(),
            KeyCode::PageUp => self.page_up(),
            KeyCode::Home => self.scroll_to_top(),
            KeyCode::End => self.scroll_to_bottom(),
            KeyCode::Char('l') => self.expand_current(),
            KeyCode::Char('h') => self.collapse_current(),
            _ => {}
        }
    }

    fn handle_mouse_event(&mut self, mouse: MouseEvent) {
        // Only handle scroll events, explicitly ignore all others
        match mouse.kind {
            MouseEventKind::ScrollDown => {
                self.scroll_down();
            }
            MouseEventKind::ScrollUp => {
                self.scroll_up();
            }
            // Explicitly ignore other mouse events (clicks, drags, moves)
            MouseEventKind::Down(_) => {}
            MouseEventKind::Up(_) => {}
            MouseEventKind::Drag(_) => {}
            MouseEventKind::Moved => {}
            _ => {}
        }
    }

    pub fn new(info: ParquetFileInfo, file_path: String) -> Self {
        Self {
            info,
            file_path,
            current_view: ViewMode::Structure,
            should_quit: false,
            scroll_offset: 0,
            max_scroll: 0,
            expanded_nodes: vec![],
        }
    }

    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    pub fn set_view(&mut self, view: ViewMode) {
        self.current_view = view;
        self.scroll_offset = 0;
    }

    pub fn next_tab(&mut self) {
        let current_index = self.current_view.index();
        let next_index = (current_index + 1) % 4;
        self.current_view = ViewMode::from_index(next_index);
        self.scroll_offset = 0;
    }

    pub fn previous_tab(&mut self) {
        let current_index = self.current_view.index();
        let previous_index = if current_index == 0 {
            3
        } else {
            current_index - 1
        };
        self.current_view = ViewMode::from_index(previous_index);
        self.scroll_offset = 0;
    }

    pub fn scroll_down(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_add(1).min(self.max_scroll);
    }

    pub fn scroll_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
    }

    pub fn page_down(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_add(10).min(self.max_scroll);
    }

    pub fn page_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(10);
    }

    pub fn scroll_to_top(&mut self) {
        self.scroll_offset = 0;
    }

    pub fn scroll_to_bottom(&mut self) {
        self.scroll_offset = usize::MAX;
    }

    pub fn expand_current(&mut self) {
        if !self.expanded_nodes.contains(&self.scroll_offset) {
            self.expanded_nodes.push(self.scroll_offset);
        }
    }

    pub fn collapse_current(&mut self) {
        self.expanded_nodes.retain(|&x| x != self.scroll_offset);
    }
}

fn render_schema_node<'a>(node: &'a SchemaNode, depth: usize, lines: &mut Vec<Line<'a>>) {
    let indent = "  ".repeat(depth);

    let mut spans = vec![Span::raw(indent)];

    if node.type_name == "GROUP" {
        spans.push(Span::styled(
            format!("{} ", node.name),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ));
        if !node.repetition.is_empty() {
            spans.push(Span::styled(
                format!("({})", node.repetition),
                Style::default().fg(Color::Gray),
            ));
        }
    } else {
        spans.push(Span::styled(
            format!("{}: ", node.name),
            Style::default().fg(Color::Cyan),
        ));
        spans.push(Span::styled(
            node.type_name.clone(),
            Style::default().fg(Color::Green),
        ));
        if !node.repetition.is_empty() {
            spans.push(Span::styled(
                format!(" ({})", node.repetition),
                Style::default().fg(Color::Gray),
            ));
        }
    }

    if let Some(logical_type) = &node.logical_type {
        spans.push(Span::styled(
            format!(" [{}]", logical_type),
            Style::default().fg(Color::Magenta),
        ));
    } else if let Some(converted_type) = &node.converted_type {
        spans.push(Span::styled(
            format!(" [{}]", converted_type),
            Style::default().fg(Color::Magenta),
        ));
    }

    lines.push(Line::from(spans));

    for child in &node.children {
        render_schema_node(child, depth + 1, lines);
    }
}
