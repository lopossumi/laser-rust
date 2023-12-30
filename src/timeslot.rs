use chrono::DateTime;
use chrono::Local;

#[derive(Clone)]
pub(crate) struct Timeslot {
    pub(crate) start: String,
    pub(crate) end: String,
}

impl Timeslot {
    pub(crate) fn duration(&self) -> i64 {
        let duration = self.end_time() - self.start_time();
        duration.num_hours()
    }

    pub(crate) fn start_time(&self) -> DateTime<Local> {
        let start_time = DateTime::parse_from_rfc3339(&self.start)
            .unwrap()
            .with_timezone(&Local);
        start_time
    }

    pub(crate) fn end_time(&self) -> DateTime<Local> {
        let end_time = DateTime::parse_from_rfc3339(&self.end)
            .unwrap()
            .with_timezone(&Local);
        end_time
    }
}

impl std::fmt::Display for Timeslot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Example output:
        // "2023-12-01 10:00 - 11:00 (1 h)"
        write!(
            f,
            "{} - {} ({} h)",
            self.start_time().format("%Y-%m-%d %H:%M"),
            self.end_time().format("%H:%M"),
            self.duration()
        )
    }
}

/// Get all remaining available times from opening times and reservations.
/// Returns a vector of Timeslot structs.
///
/// # Example
/// ```
/// let opening_times = vec![
///     Timeslot {
///         start: "2021-09-01T08:00:00+03:00".to_owned(),
///         end: "2021-09-01T16:00:00+03:00".to_owned(),
///     },
/// ];
///
/// let reservations = vec![
///     Timeslot {
///         start: "2021-09-01T10:00:00+03:00".to_owned(),
///         end: "2021-09-01T11:00:00+03:00".to_owned(),
///     }
/// ];
///
/// let available_times = get_available_times(&opening_times, &reservations);
///
/// // Expected output:
/// assert_eq!(available_times.len(), 2);
/// assert_eq!(available_times[0].start, "2021-09-01T08:00:00+03:00");
/// assert_eq!(available_times[0].end, "2021-09-01T10:00:00+03:00");
/// assert_eq!(available_times[1].start, "2021-09-01T11:00:00+03:00");
/// assert_eq!(available_times[1].end, "2021-09-01T16:00:00+03:00");
/// ```
pub(crate) fn get_available_times(
    opening_times: &Vec<Timeslot>,
    reservations: &Vec<Timeslot>,
) -> Vec<Timeslot> {
    // Iterate over each hour in opening times.
    // If the hour is not in reservations, add it to available times.
    let mut available_times: Vec<Timeslot> = Vec::new();
    for opening_time in opening_times {
        // Get start and end time of opening time
        let start_time = opening_time.start_time();
        let end_time = opening_time.end_time();

        // Iterate over each hour in opening time
        let mut current_time = start_time;
        while current_time < end_time {
            // Check if current time is in reservations
            let mut is_reserved = false;
            for reservation in reservations {
                if current_time >= reservation.start_time() && current_time < reservation.end_time()
                {
                    is_reserved = true;
                    break;
                }
            }

            // If current time is not in reservations, add it to available times
            if !is_reserved {
                let timeslot = Timeslot {
                    start: current_time.to_rfc3339(),
                    end: (current_time + chrono::Duration::hours(1)).to_rfc3339(),
                };
                available_times.push(timeslot);
            }

            // Increment current time by 1 hour
            current_time = current_time + chrono::Duration::hours(1);
        }
    }

    // Combine 1 hour timeslots into longer timeslots.
    let mut combined_timeslots: Vec<Timeslot> = Vec::new();
    let mut current_timeslot: Option<Timeslot> = None;

    for timeslot in available_times {
        if let Some(current) = current_timeslot {
            if current.end_time() == timeslot.start_time() {
                // Extend the current timeslot
                current_timeslot = Some(Timeslot {
                    start: current.start,
                    end: timeslot.end,
                });
            } else {
                // Add the current timeslot to the combined timeslots
                combined_timeslots.push(current);
                current_timeslot = Some(timeslot);
            }
        } else {
            current_timeslot = Some(timeslot);
        }
    }

    // Add the last timeslot to the combined timeslots
    if let Some(current) = current_timeslot {
        combined_timeslots.push(current);
    }

    return combined_timeslots;
}
