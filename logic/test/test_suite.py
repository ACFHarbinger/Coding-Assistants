"""
Test Runner for Multi-LLM App Test Suite

This script provides a convenient interface to run all tests or specific test modules
for the testing suite.

Usage:
    # Run all tests
    python test_suite.py
    
    # Run specific test module
    python test_suite.py --module tools
    
    # Run specific test method
    python test_suite.py --test test_write_and_read_file
"""
import subprocess
import argparse
import sys

from pathlib import Path
from typing import List, Optional

# Import definitions (ensure test_definitions.py is in the same directory)
try:
    from .test_definitions import TEST_MODULES
except ImportError:
    TEST_MODULES = {}
    print("Warning: test_definitions.py not found. Module aliases may not work.")

class PyTestRunner:
    """Manages test execution with pytest"""
    def __init__(self, test_dir: str = 'tests'):
        self.test_dir = Path(test_dir)
        self.available_modules = self._discover_test_modules()
    
    def _discover_test_modules(self) -> List[str]:
        """Discover all test modules in the test directory"""
        if not self.test_dir.exists():
            return []
        
        test_files = list(self.test_dir.glob('test_*.py'))
        return [f.stem for f in test_files]
    
    def _build_pytest_command(
        self,
        modules: Optional[List[str]] = None,
        test_class: Optional[str] = None,
        test_method: Optional[str] = None,
        verbose: bool = False,
        coverage: bool = False,
        markers: Optional[str] = None,
        failed_first: bool = False,
        maxfail: Optional[int] = None,
        capture: str = 'auto',
        tb_style: str = 'auto',
        parallel: bool = False,
        keyword: Optional[str] = None
    ) -> List[str]:
        """Build pytest command with specified options"""
        cmd = ['pytest']
        
        # Determine test targets
        if test_method:
            # Specific test method (requires module or searches all)
            if modules:
                for module in modules:
                    test_file = TEST_MODULES.get(module, f'test_{module}.py')
                    target = str(self.test_dir / test_file)
                    if test_class:
                        cmd.append(f'{target}::{test_class}::{test_method}')
                    else:
                        cmd.append(f'{target}::-k {test_method}')
            else:
                cmd.append(str(self.test_dir))
                if test_class:
                    cmd.extend(['-k', f'{test_class} and {test_method}'])
                else:
                    cmd.extend(['-k', test_method])
        
        elif test_class:
            # Specific test class
            if modules:
                for module in modules:
                    test_file = TEST_MODULES.get(module, f'test_{module}.py')
                    cmd.append(f'{self.test_dir / test_file}::{test_class}')
            else:
                cmd.append(str(self.test_dir))
                cmd.extend(['-k', test_class])
        
        elif modules:
            # Specific modules
            for module in modules:
                test_file = TEST_MODULES.get(module, f'test_{module}.py')
                cmd.append(str(self.test_dir / test_file))
        
        else:
            # All tests
            cmd.append(str(self.test_dir))
        
        # Add pytest options
        if verbose:
            cmd.append('-v')
        
        if coverage:
            cmd.extend([
                '--cov=.',
                '--cov-report=html',
                '--cov-report=term-missing'
            ])
        
        if markers:
            cmd.extend(['-m', markers])
        
        if failed_first:
            cmd.append('--ff')
        
        if maxfail:
            cmd.extend(['--maxfail', str(maxfail)])
        
        if capture != 'auto':
            cmd.append(f'--capture={capture}')
        
        if tb_style != 'auto':
            cmd.append(f'--tb={tb_style}')
        
        if parallel:
            # Requires pytest-xdist
            cmd.extend(['-n', 'auto'])
        
        if keyword:
            cmd.extend(['-k', keyword])
        
        return cmd
    
    def run_tests(self, **kwargs) -> int:
        """Execute tests with pytest"""
        cmd = self._build_pytest_command(**kwargs)
        
        print(f"Running command: {' '.join(cmd)}")
        print("=" * 80)
        
        try:
            result = subprocess.run(cmd, check=False)
            return result.returncode
        except FileNotFoundError:
            print("Error: pytest not found. Please install it with: pip install pytest")
            return 1
        except KeyboardInterrupt:
            print("\nTest execution interrupted by user")
            return 130
    
    def list_modules(self) -> None:
        """List all available test modules"""
        print("\n" + "=" * 80)
        print("Available Test Modules:")
        print("=" * 80)
        
        if not self.available_modules:
            print("No test modules found in the test directory.")
            return
        
        # Predefined modules
        print("\nPredefined modules (can use short names):")
        for short_name, filename in sorted(TEST_MODULES.items()):
            status = "✓" if filename.replace('.py', '') in self.available_modules else "✗"
            print(f"  {status} {short_name:15} -> {filename}")
        
        # Discovered modules not in predefined list
        discovered_only = set(self.available_modules) - set(f.replace('.py', '') for f in TEST_MODULES.values())
        if discovered_only:
            print("\nAdditional discovered modules:")
            for module in sorted(discovered_only):
                print(f"  ✓ {module}.py")
        
        print("\n" + "=" * 80)

def main():
    parser = argparse.ArgumentParser(description="Test Runner")
    parser.add_argument("--module", nargs="+", help="Specific modules to run")
    parser.add_argument("--test", dest="test_method", help="Specific test method to run")
    parser.add_argument("--class", dest="test_class", help="Specific test class to run")
    parser.add_argument("--verbose", action="store_true", help="Verbose output")
    parser.add_argument("--coverage", action="store_true", help="Generate coverage report")
    parser.add_argument("--list", action="store_true", help="List available test modules")
    
    args = parser.parse_args()
    
    runner = PyTestRunner()
    
    if args.list:
        runner.list_modules()
        return

    sys.exit(runner.run_tests(
        modules=args.module,
        test_method=args.test_method,
        test_class=args.test_class,
        verbose=args.verbose,
        coverage=args.coverage
    ))

if __name__ == "__main__":
    main()