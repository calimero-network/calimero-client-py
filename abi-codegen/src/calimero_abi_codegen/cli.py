"""
Command-line interface for ABI code generation.
"""

import json
import sys
from pathlib import Path
from typing import Optional
import click
from rich.console import Console
from rich.progress import Progress, SpinnerColumn, TextColumn

from .parser import WASMABIParser
from .generator import ClientGenerator


console = Console()


@click.command()
@click.option(
    "--input",
    "-i",
    type=click.Path(exists=True, path_type=Path),
    required=True,
    help="Input ABI JSON file",
)
@click.option(
    "--output",
    "-o",
    type=click.Path(path_type=Path),
    default="generated_client",
    help="Output directory for generated code",
)
@click.option(
    "--class-name",
    "-c",
    default="ABIClient",
    help="Name for the generated client class",
)
@click.option("--verbose", "-v", is_flag=True, help="Enable verbose output")
def main(input: Path, output: Path, class_name: str, verbose: bool):
    """Generate Python client code from WASM ABI schema."""

    try:
        with Progress(
            SpinnerColumn(),
            TextColumn("[progress.description]{task.description}"),
            console=console,
            transient=True,
        ) as progress:

            # Parse ABI schema
            task = progress.add_task("Parsing ABI schema...", total=None)
            parser = WASMABIParser()
            schema = parser.parse_file(input)
            progress.update(task, description="✓ ABI schema parsed")

            # Generate client code
            progress.update(task, description="Generating client code...")
            generator = ClientGenerator(schema)
            files = generator.generate_all(output, class_name)
            progress.update(task, description="✓ Client code generated")

            # Write files
            progress.update(task, description="Writing files...")
            output.mkdir(parents=True, exist_ok=True)

            for filename, content in files.items():
                file_path = output / filename
                file_path.write_text(content, encoding="utf-8")
                if verbose:
                    console.print(f"  Created: {file_path}")

            progress.update(task, description="✓ Files written")

        console.print(f"\n[green]✓ Successfully generated client code![/green]")
        console.print(f"Output directory: {output}")
        console.print(f"Client class: {class_name}")
        console.print(f"Generated files: {', '.join(files.keys())}")

        # Show usage example
        console.print(f"\n[blue]Usage example:[/blue]")
        console.print(f"```python")
        console.print(f"from calimero import create_connection, create_client")
        console.print(f"from {output.name} import {class_name}")
        console.print(f"")
        console.print(f"# Create Calimero connection and client")
        console.print(
            f"connection = create_connection(api_url='https://test.merod.dev.p2p.aws.calimero.network', node_name='test-dev-node')"
        )
        console.print(f"client = create_client(connection)")
        console.print(f"")
        console.print(
            f"# Create ABI client (you'll need a context_id and executor_public_key)"
        )
        console.print(f"abi_client = {class_name}(")
        console.print(f"    client=client,")
        console.print(f"    context_id='your-context-id',")
        console.print(f"    executor_public_key='your-executor-public-key'")
        console.print(f")")
        console.print(f"")
        console.print(f"# Use ABI methods")
        console.print(f"result = abi_client.echo_string('Hello, World!')")
        console.print(f"```")

    except Exception as e:
        console.print(f"[red]Error: {e}[/red]")
        if verbose:
            console.print_exception()
        sys.exit(1)


if __name__ == "__main__":
    main()
