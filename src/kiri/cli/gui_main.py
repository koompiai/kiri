"""kiri-popup — GTK4 voice assistant popup."""

import argparse

from kiri.config import DEFAULT_MODEL


def main():
    parser = argparse.ArgumentParser(description="Kiri voice assistant popup")
    parser.add_argument("-l", "--lang", default="en", choices=["en", "km"],
                        help="Language: en or km (default: en)")
    parser.add_argument("-m", "--model", default=DEFAULT_MODEL,
                        help=f"Model directory (default: {DEFAULT_MODEL})")
    parser.add_argument("--device", default="GPU", choices=["GPU", "CPU", "NPU"],
                        help="OpenVINO device (default: GPU)")
    parser.add_argument("--bind", metavar="KEY",
                        help="Register global shortcut (e.g. 'copilot', 'Meta+V', 'F9')")
    parser.add_argument("--unbind", action="store_true",
                        help="Remove global shortcut")
    parser.add_argument("--keybinding", action="store_true",
                        help="Show current keybinding")
    args = parser.parse_args()

    if args.bind:
        from kiri.keybind import bind_key
        bind_key(args.bind)
        return

    if args.unbind:
        from kiri.keybind import unbind_key
        unbind_key()
        return

    if args.keybinding:
        from kiri.keybind import show_binding
        show_binding()
        return

    # First-run onboarding
    from kiri.onboarding import needs_onboarding, run_onboarding
    if needs_onboarding():
        run_onboarding()

    from kiri.gui.popup import VoicePopup
    app = VoicePopup(language=args.lang, model_name=args.model, device=args.device)
    app.run([])


if __name__ == "__main__":
    main()
