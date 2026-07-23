"""
Command Line Interface for Calimero Client Python Library.
"""

import argparse
import sys

from calimero import __version__
from calimero_client_py import create_connection, create_client


def main():
    """Main CLI entry point."""
    parser = argparse.ArgumentParser(
        description="Calimero Client Python Library CLI",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  calimero-client-py --help
  calimero-client-py --version
  calimero-client-py --base-url https://test.merod.dev.p2p.aws.calimero.network list-contexts
        """,
    )

    parser.add_argument(
        "--version",
        action="version",
        version=f"calimero-client-py {__version__}",
    )

    parser.add_argument(
        "--base-url",
        default="https://test.merod.dev.p2p.aws.calimero.network",
        help="Base URL for the Calimero server (default: https://test.merod.dev.p2p.aws.calimero.network)",
    )

    parser.add_argument(
        "--node-name",
        default=None,
        help="Stable node name used to locate cached JWT tokens (required for authenticated nodes)",
    )

    parser.add_argument(
        "--auth-mode",
        choices=["none", "required"],
        default="none",
        help="Expected authentication mode; the node's actual mode is probed and a mismatch is reported (default: none)",
    )

    subparsers = parser.add_subparsers(dest="command", help="Available commands")

    # List contexts command
    subparsers.add_parser("list-contexts", help="List all contexts")

    # Add more commands here as needed

    args = parser.parse_args()

    if not args.command:
        parser.print_help()
        return

    # Create connection
    connection = create_connection(api_url=args.base_url, node_name=args.node_name)

    # Reconcile the expected auth mode against what the node actually requires.
    try:
        detected = connection.detect_auth_mode().value
        if detected != args.auth_mode:
            print(
                f"Warning: --auth-mode is '{args.auth_mode}' but the node reports "
                f"'{detected}'. Using the node's mode.",
                file=sys.stderr,
            )
    except Exception as e:  # non-fatal: proceed and let the command surface errors
        print(f"Warning: could not detect auth mode: {e}", file=sys.stderr)

    # Create client
    client = create_client(connection)

    # Execute command
    if args.command == "list-contexts":
        try:
            contexts = client.list_contexts()
            # Responses may be a bare list or wrap the list under a "data" key.
            rows = contexts.get("data") if isinstance(contexts, dict) else contexts
            count = len(rows) if hasattr(rows, "__len__") else "?"
            print(f"Found {count} contexts:")
            print(contexts)
        except Exception as e:
            print(f"Error listing contexts: {e}", file=sys.stderr)
            sys.exit(1)


def cli():
    """CLI entry point function."""
    main()


if __name__ == "__main__":
    cli()
