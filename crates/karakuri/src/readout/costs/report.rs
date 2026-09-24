use super::entry::STILL;
use super::{ms, Costs};

impl Costs {
    /// Performance and cost summary for frames drawn while the panel is untouched.
    pub(crate) fn say(
        &mut self,
        _capacity: u32,
        material: &str,
        refresh_ms: Option<f32>,
        at: (u32, u32),
    ) {
        if self.said {
            return;
        }
        self.said = true;

        let stretch = STILL.as_secs_f64();
        let fps = self.rate_over(stretch);
        let state = if self.live { "active" } else { "idle" };

        println!();
        println!("render performance ({state}, {stretch:.1}s):");
        println!(
            "  frames: {} ({fps:.1} fps), allocations: {} ({} bytes)",
            self.still.frames, self.still.allocs, self.still.bytes
        );
        println!("  composite target: {}x{} ({material})", at.0, at.1);

        if !self.frames.is_empty() {
            let mut periods: Vec<f64> = self
                .frames
                .iter()
                .filter_map(|c| c.period.map(ms))
                .collect();
            if !periods.is_empty() {
                periods.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                let median = periods[periods.len() / 2];
                let worst = *periods.last().unwrap();
                let budget_str = match refresh_ms {
                    Some(b) => format!(" / budget {b:.1} ms"),
                    None => String::new(),
                };
                println!("  frame period: median {median:.2} ms{budget_str}, worst {worst:.2} ms");
            }
        }
        println!();
    }
}
