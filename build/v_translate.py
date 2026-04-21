#!/usr/bin/env python3
"""C to V translation wrapper for GN/Bazel build systems"""

import argparse
import os
import subprocess
import sys
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(description="Translate C sources to V")
    parser.add_argument("--v-home", required=True, help="Path to V installation")
    parser.add_argument("--output-dir", required=True, help="Output directory for translated files")
    parser.add_argument("--source", action="append", dest="sources", help="C source file(s)")
    
    args = parser.parse_args()
    
    v_binary = os.path.join(args.v_home, "v")
    if not os.path.exists(v_binary):
        print(f"Error: V binary not found at {v_binary}", file=sys.stderr)
        print("Install V and set --v-home to its directory.", file=sys.stderr)
        return 1
    
    os.makedirs(args.output_dir, exist_ok=True)
    
    for source in args.sources:
        source_name = Path(source).stem
        v_output = os.path.join(args.output_dir, f"{source_name}.v")
        
        print(f"Translating {source} to {v_output}...")
        try:
            result = subprocess.run(
                [v_binary, "translate", source, "-o", v_output],
                capture_output=True,
                text=True,
                check=False
            )
            
            if result.returncode != 0:
                print(f"Warning: c2v translation had issues: {result.stderr}")
                # Create a stub V file if translation fails
                with open(v_output, "w") as f:
                    f.write(f"// Stub for {source} - translation incomplete\n")
                    f.write("module main\n\n")
                    f.write("fn placeholder() {{\n")
                    f.write("    // TODO: Complete translation\n")
                    f.write("}}\n")
            else:
                print(f"Successfully translated to {v_output}")
                
        except Exception as e:
            print(f"Error translating {source}: {e}", file=sys.stderr)
            return 1
    
    return 0


if __name__ == "__main__":
    sys.exit(main())
