pub const CSS: &str = r#"
window {
    background: linear-gradient(135deg,
        rgba(118, 56, 250, 0.92),
        rgba(200, 60, 180, 0.92),
        rgba(56, 200, 160, 0.92));
    border-radius: 16px;
}
.container { padding: 24px 32px; }
.status-label {
    color: #ffffff;
    font-size: 15px;
    font-weight: 600;
    text-shadow: 0 1px 2px rgba(0, 0, 0, 0.3);
}
.result-text {
    color: rgba(255, 255, 255, 0.92);
    font-size: 14px;
    font-style: italic;
    text-shadow: 0 1px 2px rgba(0, 0, 0, 0.3);
}
.error-label {
    color: #ffcdd2;
    font-size: 13px;
    text-shadow: 0 1px 2px rgba(0, 0, 0, 0.3);
}
.dot-recording { color: #ffffff; font-size: 24px; text-shadow: 0 0 8px rgba(255, 100, 100, 0.8); }
.dot-loading { color: rgba(255, 255, 255, 0.8); font-size: 24px; }
.dot-done { color: #a5ffd6; font-size: 24px; text-shadow: 0 0 8px rgba(100, 255, 180, 0.6); }
.level-bar-bg { background-color: rgba(255, 255, 255, 0.15); border-radius: 4px; min-height: 8px; }
.level-bar-fg { background: linear-gradient(90deg, #c83cb4, #ffffff, #38c8a0); border-radius: 4px; min-height: 8px; }
.hint-label { color: rgba(255, 255, 255, 0.5); font-size: 10px; }
.stop-btn {
    background: rgba(255, 255, 255, 0.15);
    border-radius: 8px;
    color: rgba(255, 255, 255, 0.7);
    font-size: 14px;
    font-weight: 700;
    min-width: 28px;
    min-height: 28px;
    padding: 0;
    border: none;
    box-shadow: none;
}
.stop-btn:hover { background: rgba(255, 80, 80, 0.5); color: #ffffff; }
"#;
