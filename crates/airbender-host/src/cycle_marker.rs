//! Stable host-side cycle marker snapshots collected from transpiler runs.

use std::collections::HashMap;

/// Snapshot of the cumulative cycle and delegation counters at one marker.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Mark {
    pub cycles: u64,
    pub delegations: HashMap<u32, u64>,
}

impl Mark {
    /// Derive the work performed between two cumulative marker snapshots.
    pub fn diff(&self, before: &Self) -> Self {
        let cycles = self
            .cycles
            .checked_sub(before.cycles)
            .expect("cycle markers must be compared in execution order");
        let mut delegations = HashMap::new();

        for (id, current_count) in &self.delegations {
            let diff = match before.delegations.get(id) {
                Some(previous_count) => current_count
                    .checked_sub(*previous_count)
                    .expect("delegation counters must be compared in execution order"),
                None => *current_count,
            };

            if diff != 0 {
                delegations.insert(*id, diff);
            }
        }

        Self {
            cycles,
            delegations,
        }
    }
}

#[cfg(feature = "transpiler")]
impl From<riscv_transpiler::cycle::Mark> for Mark {
    fn from(mark: riscv_transpiler::cycle::Mark) -> Self {
        Self {
            cycles: mark.cycles,
            delegations: mark.delegations,
        }
    }
}

/// All marker snapshots captured from one transpiler execution run.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CycleMarker {
    pub markers: Vec<Mark>,
    pub delegation_counter: HashMap<u32, u64>,
}

#[cfg(feature = "transpiler")]
impl From<riscv_transpiler::cycle::CycleMarker> for CycleMarker {
    fn from(marker: riscv_transpiler::cycle::CycleMarker) -> Self {
        Self {
            markers: marker.markers.into_iter().map(Mark::from).collect(),
            delegation_counter: marker.delegation_counter,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Mark;
    use std::collections::HashMap;

    #[test]
    fn diff_reports_work_between_two_markers() {
        let before = Mark {
            cycles: 8,
            delegations: HashMap::from([(0x7ca, 2), (0x7cb, 5)]),
        };
        let after = Mark {
            cycles: 13,
            delegations: HashMap::from([(0x7ca, 2), (0x7cb, 8), (0x7cc, 1)]),
        };

        let diff = after.diff(&before);

        assert_eq!(diff.cycles, 5);
        assert_eq!(diff.delegations.len(), 2);
        assert_eq!(diff.delegations.get(&0x7cb), Some(&3));
        assert_eq!(diff.delegations.get(&0x7cc), Some(&1));
    }
}
