use std::time::Duration;

#[derive(PartialEq)]
pub enum AnimationState {
    Show,
    Hide,
}

impl Default for AnimationState {
    fn default() -> AnimationState {
        AnimationState::Show
    }
}

pub struct Animation {
    pub state: AnimationState,
    transition_time: Duration,
    time_in_state: Duration,
}

impl Animation {
    pub fn new(transition_time: Duration) -> Self {
        Self {
            transition_time,
            state: AnimationState::default(),
            time_in_state: Duration::default(),
        }
    }

    pub fn add_duration(&mut self, duration: Duration) {
        self.time_in_state += duration;
        if self.time_in_state > self.transition_time {
            self.time_in_state = Duration::default();
            self.next_state();
        }
    }

    pub fn cancel(&mut self) {
        self.time_in_state = Duration::default();
        self.state = AnimationState::default();
    }

    fn next_state(&mut self) {
        use AnimationState::*;

        let new_state = match self.state {
            Show => Hide,
            Hide => Show,
        };
        self.state = new_state;
    }
}
