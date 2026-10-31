use std::io::{self, IsTerminal, Read, Write};
use std::time::{Duration, Instant};

const REDRAW_EVERY: Duration = Duration::from_millis(100);
const BAR_WIDTH: usize = 24;

/// Wraps a reader and reports how much of it has been read on stderr.
///
/// On a terminal this redraws a single line. Anywhere else (a pipe, CI logs) it prints the
/// expected size once and stays quiet, rather than filling the output with carriage returns.
pub struct ProgressReader<R> {
    inner: R,
    read: u64,
    total: Option<u64>,
    interactive: bool,
    last_draw: Option<Instant>,
}

impl<R: Read> ProgressReader<R> {
    pub fn new(inner: R, total: Option<u64>) -> Self {
        Self::with_output(inner, total, io::stderr().is_terminal())
    }

    fn with_output(inner: R, total: Option<u64>, interactive: bool) -> Self {
        if !interactive {
            if let Some(total) = total {
                eprintln!("Downloading {}...", format_bytes(total));
            }
        }
        ProgressReader {
            inner,
            read: 0,
            total,
            interactive,
            last_draw: None,
        }
    }

    #[cfg(test)]
    pub fn bytes_read(&self) -> u64 {
        self.read
    }

    /// Draw the final state and end the line. Call once the download is done.
    pub fn finish(&mut self) {
        if self.interactive {
            // an archive's reader may stop before the last trailer bytes are read
            if let Some(total) = self.total {
                self.read = self.read.max(total);
            }
            self.draw();
            eprintln!();
        }
    }

    fn draw(&mut self) {
        self.last_draw = Some(Instant::now());
        let mut stderr = io::stderr().lock();
        let _ = write!(stderr, "\r\x1b[K{}", render_line(self.read, self.total));
        let _ = stderr.flush();
    }
}

impl<R: Read> Read for ProgressReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = self.inner.read(buf)?;
        self.read += n as u64;

        let due = self.last_draw.is_none_or(|t| t.elapsed() >= REDRAW_EVERY);
        if self.interactive && due {
            self.draw();
        }
        Ok(n)
    }
}

pub fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["B", "KB", "MB", "GB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{} B", bytes)
    } else {
        format!("{:.1} {}", value, UNITS[unit])
    }
}

pub fn render_line(read: u64, total: Option<u64>) -> String {
    match total.filter(|total| *total > 0) {
        Some(total) => {
            let fraction = (read as f64 / total as f64).clamp(0.0, 1.0);
            let filled = (fraction * BAR_WIDTH as f64).round() as usize;
            format!(
                "[{}{}] {:>3}%  {} / {}",
                "#".repeat(filled),
                "-".repeat(BAR_WIDTH - filled),
                (fraction * 100.0).floor() as u64,
                format_bytes(read),
                format_bytes(total)
            )
        }
        None => format!("{} downloaded", format_bytes(read)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_sizes() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(1023), "1023 B");
        assert_eq!(format_bytes(1024), "1.0 KB");
        assert_eq!(format_bytes(1536 * 1024), "1.5 MB");
        assert_eq!(format_bytes(3 * 1024 * 1024 * 1024), "3.0 GB");
    }

    #[test]
    fn renders_a_bar() {
        assert_eq!(
            render_line(0, Some(100)),
            "[------------------------]   0%  0 B / 100 B"
        );
        assert_eq!(
            render_line(50, Some(100)),
            "[############------------]  50%  50 B / 100 B"
        );
        assert_eq!(
            render_line(100, Some(100)),
            "[########################] 100%  100 B / 100 B"
        );
    }

    #[test]
    fn copes_with_an_unknown_or_wrong_total() {
        assert_eq!(render_line(2048, None), "2.0 KB downloaded");
        assert_eq!(render_line(5, Some(0)), "5 B downloaded");
        // more data than announced must not overflow the bar
        assert!(render_line(200, Some(100)).starts_with("[########################] 100%"));
    }

    #[test]
    fn passes_data_through_and_counts_it() {
        let data = vec![7u8; 10_000];
        let mut reader = ProgressReader::with_output(&data[..], Some(10_000), false);
        let mut out = Vec::new();
        reader.read_to_end(&mut out).unwrap();
        assert_eq!(out, data);
        assert_eq!(reader.bytes_read(), 10_000);
    }
}
