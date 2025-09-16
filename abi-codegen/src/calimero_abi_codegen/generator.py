"""
Client Code Generator

Generates Python client code from WASM ABI schemas.
"""

from typing import Dict, List, Any, Optional
from pathlib import Path
from .parser import WASMABISchema, ABIMethod, ABIType
from .type_mapper import TypeMapper


class ClientGenerator:
    """Generates Python client code from WASM ABI schemas."""

    def __init__(self, schema: WASMABISchema):
        self.schema = schema
        self.type_mapper = TypeMapper(schema)

    def generate_types_file(self) -> str:
        """Generate the types.py file with all ABI type definitions."""
        lines = [
            '"""Generated types from WASM ABI schema"""',
            "",
            "from typing import Dict, List, Any, Optional, Union",
            "from dataclasses import dataclass",
            "from enum import Enum",
            "",
        ]

        # Generate dataclasses for record types
        for type_name, abi_type in self.schema.types.items():
            if abi_type.kind == "record":
                dataclass_code = self.type_mapper.generate_dataclass(abi_type)
                if dataclass_code:
                    lines.append(dataclass_code)
                    lines.append("")

        # Generate enums for variant types
        for type_name, abi_type in self.schema.types.items():
            if abi_type.kind == "variant":
                enum_code = self.type_mapper.generate_enum(abi_type)
                if enum_code:
                    lines.append(enum_code)
                    lines.append("")

        return "\n".join(lines)

    def generate_client_file(self, class_name: str = "ABIClient") -> str:
        """Generate the main client file."""
        lines = [
            '"""Generated client from WASM ABI schema"""',
            "",
            "from typing import Dict, List, Any, Optional, Union",
            "import json",
            "from calimero import Client, ClientError",
            "",
            "from .types import *",
            "",
            f"class {class_name}:",
            '    """Generated client for WASM ABI methods"""',
            "",
            "    def __init__(self, client: Client, context_id: str, executor_public_key: str):",
            '        """Initialize with a Calimero client, context ID, and executor public key"""',
            "        self.client = client",
            "        self.context_id = context_id",
            "        self.executor_public_key = executor_public_key",
            "",
        ]

        # Generate methods
        for method in self.schema.methods:
            method_code = self._generate_method(method)
            lines.append(method_code)
            lines.append("")

        return "\n".join(lines)

    def _generate_method(self, method: ABIMethod) -> str:
        """Generate a single method implementation."""
        lines = []

        # Method signature
        signature = self.type_mapper.get_method_signature(method)
        lines.append(f"    {signature}")

        # Docstring
        docstring = self.type_mapper.get_method_docstring(method)
        lines.append(docstring)

        # Method body
        lines.append("        try:")

        # Build parameters for the call
        param_names = [param.get("name", "unknown") for param in method.params]

        if param_names:
            # Create a dict with parameter names and values
            params_dict = (
                "{" + ", ".join([f'"{name}": {name}' for name in param_names]) + "}"
            )
            lines.append(f"            # Prepare arguments as JSON")
            lines.append(f"            args = json.dumps({params_dict})")
        else:
            lines.append(f"            # No parameters for this method")
            lines.append(f"            args = json.dumps({{}})")

        lines.append(f"            # Execute the ABI method")
        lines.append(f"            result = self.client.execute_function(")
        lines.append(f"                context_id=self.context_id,")
        lines.append(f"                method='{method.name}',")
        lines.append(f"                args=args,")
        lines.append(f"                executor_public_key=self.executor_public_key")
        lines.append(f"            )")

        # Handle return value
        if method.returns:
            if method.returns_nullable:
                lines.append(
                    "            return result if result is not None else None"
                )
            else:
                lines.append("            return result")
        else:
            lines.append("            return None")

        lines.append("        except ClientError as e:")
        lines.append(
            f"            raise ClientError(f'Error calling {method.name}: {{e}}')"
        )

        return "\n".join(lines)

    def generate_init_file(self, class_name: str = "ABIClient") -> str:
        """Generate __init__.py file."""
        lines = [
            '"""Generated ABI client package"""',
            "",
            f"from .client import {class_name}",
            "from .types import *",
            "",
            "__all__ = [",
            f'    "{class_name}",',
        ]

        # Add all type names
        for type_name in self.schema.types.keys():
            lines.append(f'    "{type_name}",')

        lines.extend(
            [
                "]",
                "",
                f'__version__ = "{self.schema.schema_version}"',
            ]
        )

        return "\n".join(lines)

    def generate_all(
        self, output_dir: Path, class_name: str = "ABIClient"
    ) -> Dict[str, str]:
        """Generate all files for the client package."""
        results = {}

        # Generate types.py
        types_code = self.generate_types_file()
        results["types.py"] = types_code

        # Generate client.py
        client_code = self.generate_client_file(class_name)
        results["client.py"] = client_code

        # Generate __init__.py
        init_code = self.generate_init_file(class_name)
        results["__init__.py"] = init_code

        return results
