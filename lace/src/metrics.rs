use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Default)]
pub struct Metrics {
    pub tasks: AtomicUsize,
    pub leap_success: AtomicUsize,
    pub leap_busy: AtomicUsize,
    pub leap_empty: AtomicUsize,
    pub steal_success: AtomicUsize,
    pub steal_busy: AtomicUsize,
    pub steal_empty: AtomicUsize,
    #[cfg(not(feature = "chase_lev"))]
    pub splitreqs: AtomicUsize,
    #[cfg(not(feature = "chase_lev"))]
    pub shrink_not_allstolen1: AtomicUsize,
    #[cfg(not(feature = "chase_lev"))]
    pub shrink_not_allstolen2: AtomicUsize,
    #[cfg(not(feature = "chase_lev"))]
    pub shrink_allstolen: AtomicUsize,
    #[cfg(not(feature = "chase_lev"))]
    pub shrink_fix_tail: AtomicUsize,
    #[cfg(not(feature = "chase_lev"))]
    pub grows: AtomicUsize,
}

impl std::fmt::Debug for Metrics {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        let tasks = self.tasks.load(Ordering::Relaxed);
        let leap_s = self.leap_success.load(Ordering::Relaxed);
        let leap_b = self.leap_busy.load(Ordering::Relaxed);
        let leap_e = self.leap_empty.load(Ordering::Relaxed);
        let steal_s = self.steal_success.load(Ordering::Relaxed);
        let steal_b = self.steal_busy.load(Ordering::Relaxed);
        let steal_e = self.steal_empty.load(Ordering::Relaxed);
        #[cfg(feature = "chase_lev")]
        {
            write!(
                f,
                "[tasks:{tasks}, leaps:(success:{leap_s}, busy:{leap_b}, empty:{leap_e}), steals:(success:{steal_s}, busy:{steal_b}, empty:{steal_e})]",
            )
        }
        #[cfg(not(feature = "chase_lev"))]
        {
            write!(f, "[tasks:{tasks}, leaps:(success:{leap_s}, busy:{leap_b}, empty:{leap_e}), steals:(success:{steal_s}, busy:{steal_b}, empty:{steal_e}), splitreq set:{}, grows:{}, shrinks:(as:{}, !as1:{}, !as2:{}, fix tail:{})]",
                   self.splitreqs.load(Ordering::Relaxed),
                   self.grows.load(Ordering::Relaxed),
                   self.shrink_allstolen.load(Ordering::Relaxed),
                   self.shrink_not_allstolen1.load(Ordering::Relaxed),
                   self.shrink_not_allstolen2.load(Ordering::Relaxed),
                   self.shrink_fix_tail.load(Ordering::Relaxed),
            )
        }
    }
}

impl Metrics {
    pub fn normalized(&self, factor: usize) -> Self {
        macro_rules! normalize {
            ($field:ident) => {
                AtomicUsize::new(self.$field.load(Ordering::Relaxed) / factor)
            };
        }
        Self {
            tasks: normalize!(tasks),
            leap_success: normalize!(leap_success),
            leap_busy: normalize!(leap_busy),
            leap_empty: normalize!(leap_empty),
            steal_success: normalize!(steal_success),
            steal_busy: normalize!(steal_busy),
            steal_empty: normalize!(steal_empty),
            #[cfg(not(feature = "chase_lev"))]
            splitreqs: normalize!(splitreqs),
            #[cfg(not(feature = "chase_lev"))]
            shrink_not_allstolen1: normalize!(shrink_not_allstolen1),
            #[cfg(not(feature = "chase_lev"))]
            shrink_not_allstolen2: normalize!(shrink_not_allstolen2),
            #[cfg(not(feature = "chase_lev"))]
            shrink_allstolen: normalize!(shrink_allstolen),
            #[cfg(not(feature = "chase_lev"))]
            shrink_fix_tail: normalize!(shrink_fix_tail),
            #[cfg(not(feature = "chase_lev"))]
            grows: normalize!(grows),
        }
    }
    pub fn add_into(&self, other: &mut Metrics) {
        macro_rules! process {
            () => {};
            ($field:ident $(, $rest:ident)*) => {{
                other.$field.fetch_add(self.$field.load(Ordering::Relaxed), Ordering::Relaxed);
                process!($($rest),*);
            }};
        }
        process!(
            tasks,
            leap_success,
            leap_busy,
            leap_empty,
            steal_success,
            steal_busy,
            steal_empty
        );
        #[cfg(not(feature = "chase_lev"))]
        {
            process!(
                splitreqs,
                shrink_not_allstolen1,
                shrink_not_allstolen2,
                shrink_allstolen,
                shrink_fix_tail,
                grows
            );
        }
    }
}
