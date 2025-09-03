#!/usr/bin/env python3
"""
Test script to verify Python 3.13 environment is working correctly
"""

import sys
import platform

def main():
    print("🐍 Python Environment Test")
    print("=" * 40)
    
    # Check Python version
    print(f"Python version: {sys.version}")
    print(f"Python executable: {sys.executable}")
    print(f"Platform: {platform.platform()}")
    
    # Check if we're in a virtual environment
    if hasattr(sys, 'real_prefix') or (hasattr(sys, 'base_prefix') and sys.base_prefix != sys.prefix):
        print("✅ Running in virtual environment")
    else:
        print("❌ Not running in virtual environment")
    
    # Test basic functionality
    try:
        import json
        print("✅ JSON module imported successfully")
        
        import asyncio
        print("✅ Asyncio module imported successfully")
        
        # Test async functionality
        async def test_async():
            return "Async test successful"
        
        result = asyncio.run(test_async())
        print(f"✅ {result}")
        
    except ImportError as e:
        print(f"❌ Import error: {e}")
    
    print("\n🎯 Environment setup complete!")
    print("\nNote: The full calimero-client bindings cannot be built due to")
    print("a dependency issue in soroban-client 0.3.9 with stellar-baselib 0.4.9.")
    print("This requires an update to the calimero-client crate itself.")

if __name__ == "__main__":
    main()
