"""kiri-popup — GTK4 voice assistant popup."""

import argparse

from kiri.config import DEFAULT_MODEL
from kiri.gui.popup import VoicePopup


def main():
    parser = argparse.ArgumentParser(description="Kiri voice assistant popup")
    parser.add_argument("-l", "--lang", default="en", choices=["en", "km"],
                        help="Language: en or km (default: en)")
    parser.add_argument("-m", "--model", default=DEFAULT_MODEL,
                        help=f"Model directory (default: {DEFAULT_MODEL})")
    parser.add_argument("--device", default="GPU", choices=["GPU", "CPU"],
                        help="OpenVINO device (default: GPU)")
    args = parser.parse_args()

    app = VoicePopup(language=args.lang, model_name=args.model, device=args.device)
    app.run([])


if __name__ == "__main__":
    main()
