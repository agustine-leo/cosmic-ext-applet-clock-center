// SPDX-License-Identifier: GPL-3.0-only

use crate::config::TimeAppletConfig;

/// Text for the panel button. Placeholder until the clock task lands.
pub fn format_label(_now: &jiff::Zoned, _cfg: &TimeAppletConfig) -> String {
    String::new()
}
