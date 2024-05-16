use core::marker::PhantomData;
use embedded_hal::digital::InputPin;
use embedded_time::duration::Milliseconds;
use embedded_time::{Clock, Instant};

/// UI Switch
///
/// This advanced switch tracks it's own state and can provide
/// some additional information on it's current state, how long
/// the current state is in effect and how long the last state was in
/// effect.
///
/// Implementors of this trait should own their resources.
/// TODO: implement ability to set the default state for the user
pub trait Switch<C: Clock> {
    /// Reset the switch state to initial values
    fn reset(&mut self);

    /// Poll the switch for the hardware state changes
    ///
    /// This must be done in regular intervals in order to make this abstraction
    /// work properly. There might be limits on what this abstraction can track based
    /// on how small / large the intervals are.
    fn poll(&mut self, now: Instant<C>);

    /// Indicates that the switch state has has_changed since the last poll
    fn has_changed(&self) -> bool;

    /// Indicates that the switch is in pressed state
    fn is_pressed(&self) -> bool;

    /// Indicates that the switch is in released state
    fn is_released(&self) -> bool;

    /// Returns the duration for which the switch has been pressed for
    ///
    /// This is a wrapper for [last_state_lasted_for](#method.last_state_lasted_for)
    /// that conveniently returns [`None`] if current state is not the opposite
    /// of the state the function name indicates
    ///
    /// Returns [`None`] if the switch is not in released state
    fn pressed_for(&self) -> Option<Milliseconds<C::T>>;

    /// Returns the duration for which the switch has been released for
    ///
    /// This is a wrapper for [last_state_lasted_for](#method.last_state_lasted_for)
    /// that conveniently returns [`None`] if current state is not the opposite
    /// of the state the function name indicates
    ///
    /// Returns [`None`] if the switch is not in the pressed state
    fn released_for(&self) -> Option<Milliseconds<C::T>>;

    /// Returns the duration for which the last switch state lasted for
    fn prev_state_lasted_for(&self) -> Milliseconds<C::T>;

    /// Returns the duration for which the current state is held
    ///
    /// Requires an instant to be passed in to compare against the switch state
    fn current_state(&self, now: Instant<C>) -> Milliseconds<C::T>;

    /// Wait for the state to change
    ///
    /// Polls the switch until it's state has been changed
    ///
    /// This operation is blocking
    fn wait(&mut self, clock: &C);
}

pub mod switch_state {
    use embedded_hal::digital::InputPin;

    /// [`PressedState`] defines the switch behavior on pin raw values
    ///
    /// This is internally implemented for [`PressedOnHigh`] and [`PressedOnLow`]
    /// to implement different behaviors.
    pub trait PressedState {
        fn get_pressed_state<P: InputPin>(pin: &mut P) -> bool;
    }

    /// Sets the switch behavior to be in pressed state when the pin is high
    pub struct PressedOnHigh {}

    /// Sets the switch behavior to be in pressed state when the pin is is low
    pub struct PressedOnLow {}

    impl PressedState for PressedOnHigh {
        fn get_pressed_state<P: InputPin>(pin: &mut P) -> bool {
            pin.is_high().unwrap()
        }
    }
    impl PressedState for PressedOnLow {
        fn get_pressed_state<P: InputPin>(pin: &mut P) -> bool {
            pin.is_low().unwrap()
        }
    }
}

// TODO: instead of bools check if we can use bitflags crate to get more efficient and ergonomic

/// Switch implementation for [`InputPin`] of `embedded_hal`
pub struct PinSwitch<P: InputPin, S: switch_state::PressedState, C: Clock> {
    pin: P,
    is_pressed: bool,
    has_changed: bool,
    last_change_at: Instant<C>,
    prev_state_lasted: Milliseconds<C::T>,
    pressed_state: PhantomData<S>,
}

impl<P: InputPin, S: switch_state::PressedState, C: Clock> PinSwitch<P, S, C> {
    /// Create new [`PinSwitch`] instance for the passed in `pin`
    pub fn new(pin: P) -> Self {
        Self {
            pin,
            is_pressed: false,
            has_changed: false,
            last_change_at: Instant::<C>::new(C::T::from(0)),
            prev_state_lasted: Milliseconds::<C::T>::new(C::T::from(0)),
            pressed_state: Default::default(),
        }
    }
}

impl<P: InputPin, S: switch_state::PressedState, C: Clock> Switch<C> for PinSwitch<P, S, C> {
    fn poll(&mut self, now: Instant<C>) {
        let new_state = S::get_pressed_state(&mut self.pin);

        if new_state == self.is_pressed {
            self.has_changed = false;
            return;
        }

        self.is_pressed = new_state;
        self.has_changed = true;
        self.prev_state_lasted = self.current_state(now);
        self.last_change_at = now;
    }

    fn has_changed(&self) -> bool {
        self.has_changed
    }

    fn is_pressed(&self) -> bool {
        self.is_pressed
    }

    fn is_released(&self) -> bool {
        !self.is_pressed
    }

    fn pressed_for(&self) -> Option<Milliseconds<C::T>> {
        if !self.is_pressed {
            return Some(self.prev_state_lasted);
        }
        None
    }

    fn released_for(&self) -> Option<Milliseconds<C::T>> {
        if self.is_pressed {
            return Some(self.prev_state_lasted);
        }
        None
    }

    fn wait(&mut self, clock: &C) {
        loop {
            self.poll(clock.try_now().unwrap());
            if self.has_changed {
                break;
            }
        }
    }

    fn reset(&mut self) {
        self.last_change_at = Instant::<C>::new(C::T::from(0));
        self.prev_state_lasted = Milliseconds::<C::T>::new(C::T::from(0));
        self.has_changed = false;
        self.is_pressed = false;
    }

    fn prev_state_lasted_for(&self) -> Milliseconds<<C as Clock>::T> {
        self.prev_state_lasted
    }

    fn current_state(&self, now: Instant<C>) -> Milliseconds<<C as Clock>::T> {
        now
            .checked_duration_since(&self.last_change_at)
            .unwrap()
            .try_into()
            .unwrap()
    }
}
