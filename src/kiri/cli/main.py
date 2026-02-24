"""kiri — CLI voice-to-text transcriber."""

import argparse
import sys

from kiri.audio.recorder import AudioRecorder
from kiri.config import DEFAULT_MODEL, WHISPER_RATE
from kiri.output.clipboard import copy_to_clipboard
from kiri.output.notes import save_to_notes
from kiri.transcription.engine import WhisperEngine
from kiri.transcription.models import check_model


def main():
    parser = argparse.ArgumentParser(description="Kiri — voice-to-text assistant")
    parser.add_argument("-f", "--file", help="Target markdown file (without .md)")
    parser.add_argument("-d", "--duration", type=int, default=60,
                        help="Max recording duration in seconds (default: 60)")
    parser.add_argument("-l", "--language", default="en",
                        choices=["en", "km"],
                        help="Language: en=English, km=Khmer (default: en)")
    parser.add_argument("--clipboard", action="store_true",
                        help="Copy to clipboard instead of saving to file")
    parser.add_argument("--device", default="GPU",
                        choices=["GPU", "CPU"],
                        help="OpenVINO device (default: GPU)")
    parser.add_argument("-m", "--model", default=DEFAULT_MODEL,
                        help=f"Model directory name (default: {DEFAULT_MODEL})")
    parser.add_argument("--check", action="store_true",
                        help="Check available OpenVINO devices and exit")
    args = parser.parse_args()

    if args.check:
        import openvino as ov
        core = ov.Core()
        print("Available devices:", core.available_devices)
        return

    if not check_model(args.model):
        sys.exit(1)

    # Load model first, then record — so no speech is missed
    engine = WhisperEngine(model_name=args.model, device=args.device)
    engine.load()

    recorder = AudioRecorder()
    audio = recorder.record_fixed(duration=args.duration)

    if len(audio) < WHISPER_RATE:
        print("\u26a0\ufe0f  Recording too short, discarding.")
        sys.exit(1)

    text = engine.transcribe(audio, language=args.language)

    if not text:
        print("\u26a0\ufe0f  No speech detected.")
        sys.exit(1)

    print(f"\n\U0001f4dd Transcribed:\n{text}\n")

    if args.clipboard:
        copy_to_clipboard(text)
    else:
        filepath = save_to_notes(text, filename=args.file)
        print(f"\u2705 Saved to {filepath}")


if __name__ == "__main__":
    main()
