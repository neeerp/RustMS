//! Temporary skill effects and skill cooldowns. They last only while the
//! character is online and are never saved.

use std::time::{Duration, Instant};

/// How often Recovery heals.
pub const RECOVERY_INTERVAL: Duration = Duration::from_secs(5);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BuffStat {
    /// Heals `value` HP every `RECOVERY_INTERVAL`.
    Recovery,
    Speed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Buff {
    pub skill_id: i32,
    pub stat: BuffStat,
    pub value: i16,
    pub expires: Instant,
    next_heal: Option<Instant>,
}

impl Buff {
    pub fn new(
        skill_id: i32,
        stat: BuffStat,
        value: i16,
        now: Instant,
        duration: Duration,
    ) -> Self {
        Self {
            skill_id,
            stat,
            value,
            expires: now + duration,
            next_heal: (stat == BuffStat::Recovery).then(|| now + RECOVERY_INTERVAL),
        }
    }

    fn next_deadline(&self) -> Instant {
        self.next_heal
            .map_or(self.expires, |at| at.min(self.expires))
    }
}

/// What happened when the buffs were brought up to date.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct BuffTick {
    pub expired: Vec<BuffStat>,
    /// HP that Recovery heals now.
    pub heal: i32,
}

#[derive(Clone, Debug, Default)]
pub struct Buffs {
    active: Vec<Buff>,
    /// (skill id, usable again at)
    cooldowns: Vec<(i32, Instant)>,
}

impl Buffs {
    /// Start a buff, replacing one on the same stat.
    pub fn give(&mut self, buff: Buff) {
        self.active.retain(|b| b.stat != buff.stat);
        self.active.push(buff);
    }

    pub fn active(&self) -> &[Buff] {
        &self.active
    }

    pub fn cooling_down(&self, skill_id: i32, now: Instant) -> bool {
        self.cooldowns
            .iter()
            .any(|&(id, until)| id == skill_id && until > now)
    }

    pub fn start_cooldown(&mut self, skill_id: i32, now: Instant, length: Duration) {
        self.cooldowns
            .retain(|&(id, until)| id != skill_id && until > now);
        self.cooldowns.push((skill_id, now + length));
    }

    /// When `advance` next has something to do.
    pub fn next_deadline(&self) -> Option<Instant> {
        self.active.iter().map(Buff::next_deadline).min()
    }

    /// Heal for each Recovery interval that has passed, then end the buffs
    /// that ran out.
    pub fn advance(&mut self, now: Instant) -> BuffTick {
        let mut tick = BuffTick::default();
        for buff in &mut self.active {
            while let Some(at) = buff.next_heal.filter(|&at| at <= now && at <= buff.expires) {
                tick.heal += i32::from(buff.value);
                buff.next_heal = Some(at + RECOVERY_INTERVAL);
            }
        }
        self.active.retain(|buff| {
            let running = buff.expires > now;
            if !running {
                tick.expired.push(buff.stat);
            }
            running
        });
        tick
    }

    /// End the buffs a skill gave, e.g. when the player right-clicks its
    /// icon. Its cooldown keeps running.
    pub fn cancel(&mut self, skill_id: i32) -> Vec<BuffStat> {
        let mut ended = Vec::new();
        self.active.retain(|buff| {
            let cancelled = buff.skill_id == skill_id;
            if cancelled {
                ended.push(buff.stat);
            }
            !cancelled
        });
        ended
    }

    /// End every buff, e.g. on death. Cooldowns keep running.
    pub fn clear(&mut self) -> Vec<BuffStat> {
        self.active.drain(..).map(|buff| buff.stat).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECOND: Duration = Duration::from_secs(1);

    #[test]
    fn recovery_heals_every_five_seconds_until_it_ends() {
        let start = Instant::now();
        let mut buffs = Buffs::default();
        buffs.give(Buff::new(1001, BuffStat::Recovery, 4, start, 12 * SECOND));
        assert_eq!(buffs.next_deadline(), Some(start + 5 * SECOND));
        assert_eq!(buffs.advance(start + 4 * SECOND), BuffTick::default());
        assert_eq!(buffs.advance(start + 5 * SECOND).heal, 4);
        assert_eq!(buffs.next_deadline(), Some(start + 10 * SECOND));
        // Missed ticks are caught up; it ends before the third heal.
        let tick = buffs.advance(start + 13 * SECOND);
        assert_eq!((tick.heal, tick.expired), (4, vec![BuffStat::Recovery]));
        assert_eq!(buffs.next_deadline(), None);
    }

    #[test]
    fn a_new_buff_replaces_one_on_the_same_stat() {
        let start = Instant::now();
        let mut buffs = Buffs::default();
        buffs.give(Buff::new(1002, BuffStat::Speed, 10, start, 4 * SECOND));
        buffs.give(Buff::new(1001, BuffStat::Recovery, 4, start, 30 * SECOND));
        buffs.give(Buff::new(1002, BuffStat::Speed, 20, start, 12 * SECOND));
        assert_eq!(buffs.active().len(), 2);
        assert_eq!(buffs.next_deadline(), Some(start + 5 * SECOND));
        assert!(buffs.advance(start + 4 * SECOND).expired.is_empty());
        assert_eq!(buffs.clear(), vec![BuffStat::Recovery, BuffStat::Speed]);
    }

    #[test]
    fn cancelling_a_skill_ends_only_its_buffs() {
        let start = Instant::now();
        let mut buffs = Buffs::default();
        buffs.give(Buff::new(1001, BuffStat::Recovery, 4, start, 30 * SECOND));
        buffs.give(Buff::new(1002, BuffStat::Speed, 20, start, 12 * SECOND));
        buffs.start_cooldown(1002, start, 60 * SECOND);
        assert_eq!(buffs.cancel(1002), vec![BuffStat::Speed]);
        assert_eq!(buffs.cancel(1002), vec![]);
        assert_eq!(buffs.active()[0].stat, BuffStat::Recovery);
        assert!(buffs.cooling_down(1002, start + SECOND));
    }

    #[test]
    fn cooldowns_run_out() {
        let start = Instant::now();
        let mut buffs = Buffs::default();
        buffs.start_cooldown(1002, start, 60 * SECOND);
        assert!(buffs.cooling_down(1002, start + 59 * SECOND));
        assert!(!buffs.cooling_down(1001, start));
        assert!(!buffs.cooling_down(1002, start + 60 * SECOND));
    }
}
