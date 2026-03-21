pub const REQUIRED_SESSION_FILES: [&str; 9] = [
    "workspace.yaml",
    "SESSION.md",
    "AGENTS.md",
    "context/MEMORY.md",
    "context/PLAN.md",
    "context/CONSTRAINTS.md",
    "context/DECISIONS.md",
    "context/LINKS.md",
    ".opencode/opencode.jsonc",
];

pub const SESSION_MARKERS: [&str; 4] = [
    "workspace.yaml",
    "SESSION.md",
    "AGENTS.md",
    "context/PLAN.md",
];

pub const ALLOWED_GLOBAL_SKILLS_PATHS: [&str; 2] =
    ["~/.taskspace/SKILLS.md", "~/.config/taskspace/SKILLS.md"];

pub fn default_instructions() -> [&'static str; 6] {
    [
        "SESSION.md",
        "AGENTS.md",
        "~/.taskspace/SKILLS.md",
        "context/CONSTRAINTS.md",
        "context/MEMORY.md",
        "context/PLAN.md",
    ]
}
