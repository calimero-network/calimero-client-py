import base58
from typing import Tuple
import nacl.signing
import nacl.encoding

class Ed25519Keypair:
    def __init__(self, private_key: bytes):
        """Initialize with private key bytes."""
        self._signing_key = nacl.signing.SigningKey(private_key)
        self._verify_key = self._signing_key.verify_key

    @classmethod
    def from_base58(cls, base58_private_key: str) -> 'Ed25519Keypair':
        """Create from base58-encoded private key."""
        private_key = base58.b58decode(base58_private_key)
        return cls(private_key)

    @property
    def public_key(self) -> bytes:
        """Get the public key."""
        return self._verify_key.encode()

    @property
    def public_key_base58(self) -> str:
        """Get the base58-encoded public key."""
        return base58.b58encode(self.public_key).decode()

    def sign(self, message: str) -> bytes:
        """Sign a message."""
        return self._signing_key.sign(message.encode()).signature

    def sign_base58(self, message: str) -> str:
        """Sign a message and return base58-encoded signature."""
        signature = self.sign(message)
        return base58.b58encode(signature).decode()

    def verify(self, message: str, signature: bytes) -> bool:
        """Verify a signature."""
        try:
            self._verify_key.verify(message.encode(), signature)
            return True
        except nacl.exceptions.BadSignatureError:
            return False

    def verify_base58(self, message: str, base58_signature: str) -> bool:
        """Verify a base58-encoded signature."""
        signature = base58.b58decode(base58_signature)
        return self.verify(message, signature) 