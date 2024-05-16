use embedded_hal::digital::{PinState, StatefulOutputPin};
use embedded_time::duration::Milliseconds;
use embedded_time::rate::Rate;
use embedded_time::{Clock, Instant};

use self::effects::LedEffect;

pub mod effects {
    use embedded_time::{duration::Milliseconds, rate::Hertz, Clock, Instant, TimeInt};

    /// LED Effect type
    #[derive(Copy, Clone, Debug)]
    pub enum EffectType<T: TimeInt = u32> {
        /// Single pulse. Effects does not repeat
        Pulse(Milliseconds<T>),
        /// Blink at given Hz value
        Blink(Hertz<T>),
    }

    /// LED Effect instance
    ///
    /// Stores some additional metadata alongside with the effect type
    /// used during the effect processing.
    #[derive(Copy, Clone, Debug)]
    pub struct LedEffect<C: Clock> {
        current_cycle_started_at: Option<Instant<C>>,
        started_at: Option<Instant<C>>,
        duration: Option<Milliseconds<C::T>>,
        fx_type: EffectType<C::T>,
    }

    impl<C: Clock> LedEffect<C> {
        /// Create new LED Effect instance with the assigned effect type
        pub fn new(fx_type: EffectType<C::T>) -> Self {
            Self {
                current_cycle_started_at: None,
                fx_type,
                duration: None,
                started_at: None
            }
        }

        /// Indicates whether the effect has started
        pub fn has_started(&self) -> bool {
            self.started_at.is_some()
        }

        /// Get the timestamp at which the effect started
        pub fn started_at(&self) -> Option<Instant<C>> {
            self.started_at
        }

        /// Sets the effect start point
        ///
        /// This should be only called by the poll functions of [`Led`] implementations
        pub fn set_started_at(&mut self, now: Instant<C>) {
            self.started_at = Some(now);
            self.current_cycle_started_at = self.started_at;
        }

        /// Returns the effect type of this LED effect instance
        pub fn get_type(&self) -> &EffectType<C::T> {
            &self.fx_type
        }

        /// Returns the duration for which the effect should last
        pub fn get_duration(&self) -> Option<Milliseconds<C::T>> {
            self.duration
        }

        /// Sets the effect duration
        pub fn set_duration(&mut self, dur: Milliseconds<C::T>) {
            self.duration = Some(dur)
        }

        /// Returns elapsed duration since the effect has started
        pub fn time_elapsed(&self, now: Instant<C>) -> Option<Milliseconds<C::T>> {
            if let Some(started_at) = &self.started_at {
                return now
                    .checked_duration_since(started_at)
                    .map(|d| Milliseconds::<C::T>::try_from(d).unwrap());
            }
            None
        }

        /// Returns the duration of current cycle
        pub fn current_cycle_duration(&self, now: Instant<C>) -> Option<Milliseconds<C::T>> {
            if let Some(started_at) = &self.current_cycle_started_at {
                return now
                    .checked_duration_since(started_at)
                    .map(|d| Milliseconds::<C::T>::try_from(d).unwrap());
            }
            None
        }

        /// Start new cycle at an timestamp
        pub fn start_new_cycle(&mut self, now: Instant<C>) {
            self.current_cycle_started_at = Some(now);
        }
    }

    #[inline]
    pub fn pulse<C: Clock>(duration_ms: u16) -> EffectType<C::T> {
        let v = C::T::from(duration_ms.into());
        EffectType::Pulse::<C::T>(Milliseconds::<C::T>::new(v))
    }

    #[inline]
    pub fn blink<C: Clock>(rate_hz: u8) -> EffectType<C::T> {
        let v = C::T::from(rate_hz.into());
        EffectType::Blink::<C::T>(Hertz::<C::T>::new(v))
    }
}

/// UI LED
///
/// This LED abstraction tracks it's status and provides
/// an interface for setting visual effects such as blinking
/// on the LED.
///
/// Implementors should own their resources
/// TODO: implement ability to set the default state for the user
pub trait Led<C: Clock> {
    // Indicates whether the current state is on or off
    fn is_on(&mut self) -> bool;

    /// Turns on the LED
    ///
    /// Has no effect if the LED is already turned on
    fn turn_on(&mut self);

    /// Turns off the LED
    ///
    /// has no effect if the LED is already turned off
    fn turn_off(&mut self);

    /// Toggles the led on/off
    fn toggle(&mut self);

    /// Sets the effect on this LED instance
    ///
    /// By default effect will have infinite duration unless set otherwise by
    /// [set_effect_duration](#method.set_effect_duration) call
    ///
    /// Setting the effect while another one is active will overwrite it on the next
    /// [poll](#method.poll) call
    fn set_effect(&mut self, effect: effects::LedEffect<C>);

    /// Sets the current effect duration on this LED instance
    ///
    /// Can be used to prolong current effect duration
    ///
    /// Does nothing if no effect is currently in place
    fn set_effect_duration(&mut self, dur: Milliseconds<C::T>);

    /// Returns the current LED effect
    ///
    /// Returns [`None`] if no effect is in place
    fn get_effect(&self) -> Option<&LedEffect<C>>;

    /// Clears current the effect
    ///
    /// This should also revert the LED to the state it was in
    /// before the effect took place
    fn clear_effect(&mut self);

    /// Polls the LED, updating it's state tracking and hardware state
    ///
    /// This must be done in regular intervals in order to make this abstraction
    /// work properly. There might be limits on what this abstraction can track based
    /// on how small / large the intervals are.
    fn poll(&mut self, now: Instant<C>);
}

pub struct PinLed<P: StatefulOutputPin, C: Clock> {
    pin: P,
    effect: Option<effects::LedEffect<C>>,
    is_on: bool,
}

impl<P: StatefulOutputPin, C: Clock> PinLed<P, C> {
    pub fn new(pin: P) -> Self {
        Self { pin, effect: None, is_on: false }
    }
}

impl<P: StatefulOutputPin, C: Clock> Led<C> for PinLed<P, C> {
    fn is_on(&mut self) -> bool {
        self.is_on
    }

    fn turn_on(&mut self) {
        self.is_on = true;
    }

    fn turn_off(&mut self) {
        self.is_on = false;
    }

    fn toggle(&mut self) {
        self.is_on = !self.is_on;
    }

    fn set_effect(&mut self, effect: effects::LedEffect<C>) {
        self.effect = Some(effect);
    }

    fn set_effect_duration(&mut self, dur: Milliseconds<<C as Clock>::T>) {
        if let Some(fx) = &mut self.effect {
            fx.set_duration(dur)
        }
    }

    fn clear_effect(&mut self) {
        self.effect = None;
        self.turn_off();
    }

    fn poll(&mut self, now: Instant<C>) {
        if let Some(fx) = &mut self.effect {
            // LED has an effect, process effect

            let elapsed = fx.time_elapsed(now);

            // check if effect should finish
            if let Some(fx_dur) = fx.get_duration() {
                if let Some(elapsed) = elapsed {
                    if elapsed > fx_dur {
                        // effect is over
                        self.clear_effect();
                        self.pin.set_low().unwrap();
                        return;
                    }
                }
            }

            let mut clear_effect = false;

            match fx.get_type() {
                effects::EffectType::Pulse(dur) => {
                    if let Some(current_dur) = fx.current_cycle_duration(now) {
                        if current_dur > *dur {
                            // effect is over
                            clear_effect = true;
                            self.pin.set_low().unwrap();
                        } else if ! fx.has_started() {
                            self.pin.set_high().unwrap();
                        }
                    }
                }
                effects::EffectType::Blink(rate) => {
                    if let Some(current_dur) = fx.current_cycle_duration(now) {
                        if current_dur > rate.to_duration::<Milliseconds<C::T>>().unwrap() {
                            // toggle the led on/off on each state change
                            if self.pin.is_set_low().unwrap() {
                                self.pin.set_high().unwrap();
                            } else {
                                self.pin.set_low().unwrap();
                            }
                            fx.start_new_cycle(now);
                        }
                    }
                }
            }

            if clear_effect {
                self.clear_effect();
                return;
            }

            // Effect is just starting, save current timestamp
            if ! fx.has_started() {
                fx.set_started_at(now);
            }
        } else {
            // No effect on led, proceed as normal
            let state = self.is_on;
            self.pin
                .set_state(match state {
                    false => PinState::High,
                    true => PinState::Low,
                })
                .unwrap()
        }
    }

    fn get_effect(&self) -> Option<&LedEffect<C>> {
        self.effect.as_ref()
    }
}
