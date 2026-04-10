use crate::utils::ids::NEXT_ID;

pub const GROUP_COLORS: &[&str] = &[
    "#8b5cf6", "#22d3ee", "#f59e0b", "#10b981", "#ef4444", "#ec4899", "#6366f1", "#14b8a6",
];

pub fn random_group_color() -> String {
    let n = NEXT_ID.load(std::sync::atomic::Ordering::Relaxed);
    GROUP_COLORS[n % GROUP_COLORS.len()].into()
}
