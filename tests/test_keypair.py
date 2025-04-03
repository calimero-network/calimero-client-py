import pytest
import nacl.signing
import base58
from calimero import Ed25519Keypair

def test_keypair_creation(ed25519_keypair):
    """Test keypair creation and properties."""
    assert ed25519_keypair is not None
    assert isinstance(ed25519_keypair.public_key, bytes)
    assert isinstance(ed25519_keypair.public_key_base58, str)

def test_keypair_from_base58():
    """Test creating keypair from base58-encoded private key."""
    # Generate a new keypair
    signing_key = nacl.signing.SigningKey.generate()
    private_key_base58 = base58.b58encode(signing_key._signing_key).decode()
    
    # Create keypair from base58
    keypair = Ed25519Keypair.from_base58(private_key_base58)
    assert keypair is not None
    assert keypair.public_key == signing_key.verify_key.encode()

def test_sign_and_verify(ed25519_keypair):
    """Test signing and verifying messages."""
    message = "test message"
    
    # Sign message
    signature = ed25519_keypair.sign(message)
    assert isinstance(signature, bytes)
    
    # Verify signature
    assert ed25519_keypair.verify(message, signature)
    
    # Test with different message
    assert not ed25519_keypair.verify("different message", signature)

def test_sign_and_verify_base58(ed25519_keypair):
    """Test signing and verifying messages with base58 encoding."""
    message = "test message"
    
    # Sign message and get base58 signature
    signature_base58 = ed25519_keypair.sign_base58(message)
    assert isinstance(signature_base58, str)
    
    # Verify base58 signature
    assert ed25519_keypair.verify_base58(message, signature_base58)
    
    # Test with different message
    assert not ed25519_keypair.verify_base58("different message", signature_base58)

def test_invalid_base58():
    """Test handling of invalid base58 input."""
    with pytest.raises(ValueError):
        Ed25519Keypair.from_base58("invalid_base58_string") 