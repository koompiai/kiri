"""WhisperEngine — load and transcribe with OpenVINO."""

import numpy as np

from kiri.config import WHISPER_RATE
from kiri.transcription.models import model_path


class WhisperEngine:
    """Manages an OpenVINO Whisper model for transcription."""

    def __init__(self, model_name: str = "whisper-large-v3-ov", device: str = "GPU"):
        self.model_name = model_name
        self.device = device
        self.processor = None
        self.model = None

    def load(self):
        """Load processor and model. Call before transcribe()."""
        from optimum.intel import OVModelForSpeechSeq2Seq
        from transformers import AutoProcessor

        path = model_path(self.model_name)
        print(f"\U0001f504 Loading {self.model_name} on {self.device}...")
        self.processor = AutoProcessor.from_pretrained(str(path))
        self.model = OVModelForSpeechSeq2Seq.from_pretrained(
            str(path), device=self.device,
        )
        print("\u2705 Model ready.")

    def transcribe(self, audio: np.ndarray, language: str = "en") -> str:
        """Transcribe 16 kHz float32 audio. Returns text."""
        lang_name = {"en": "English", "km": "Khmer"}.get(language, language)
        print(f"\U0001f9e0 Transcribing ({lang_name})...")

        inputs = self.processor(
            audio, sampling_rate=WHISPER_RATE, return_tensors="pt",
        )

        generate_kwargs: dict = {}
        try:
            forced_decoder_ids = self.processor.get_decoder_prompt_ids(
                language=language, task="transcribe",
            )
            generate_kwargs["forced_decoder_ids"] = forced_decoder_ids
        except Exception:
            pass  # finetuned models may not support this

        try:
            predicted_ids = self.model.generate(
                inputs["input_features"], **generate_kwargs,
            )
        except ValueError:
            generate_kwargs.pop("forced_decoder_ids", None)
            predicted_ids = self.model.generate(
                inputs["input_features"], **generate_kwargs,
            )

        return self.processor.batch_decode(
            predicted_ids, skip_special_tokens=True,
        )[0].strip()
