from enum import Enum


class AppAttestationPurpose(str, Enum):
    KMS = "kms"
    RECEIPTS = "receipts"

    def __str__(self) -> str:
        return str(self.value)
