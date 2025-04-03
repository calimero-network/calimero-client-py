import os
from pathlib import Path
from typing import Optional
import toml
from pydantic import BaseModel

class Config(BaseModel):
    node_url: str
    application_id: str
    executor_public_key: str
    keypair: Optional['Ed25519Keypair'] = None

    @classmethod
    def load_from_file(cls, config_path: str) -> 'Config':
        """Load configuration from a TOML file."""
        with open(config_path, 'r') as f:
            config_data = toml.load(f)
        
        return cls(
            node_url=config_data['node']['url'],
            application_id=config_data['application']['id'],
            executor_public_key=config_data['executor']['public_key']
        )

    @classmethod
    def load_from_env(cls) -> 'Config':
        """Load configuration from environment variables."""
        return cls(
            node_url=os.getenv('CALIMERO_NODE_URL', 'http://localhost:2428'),
            application_id=os.getenv('CALIMERO_APPLICATION_ID', ''),
            executor_public_key=os.getenv('CALIMERO_EXECUTOR_PUBLIC_KEY', '')
        )

    @classmethod
    def get_default_config_path(cls) -> Path:
        """Get the default configuration path."""
        home = Path.home()
        return home / '.calimero' / 'config.toml' 