#!/usr/bin/env python3
"""
Swift Code Quality Checker - Alternative to SwiftLint when SourceKit is unavailable
Performs basic Swift code quality checks without requiring SourceKit/Xcode
"""

import os
import re
import sys
import glob
from pathlib import Path

class SwiftChecker:
    def __init__(self):
        self.issues = []
        self.warnings = 0
        self.errors = 0
    
    def log_issue(self, file_path, line_num, severity, rule, message):
        """Log a code quality issue"""
        issue = f"{file_path}:{line_num}: {severity}: {message} ({rule})"
        self.issues.append(issue)
        if severity == "error":
            self.errors += 1
        else:
            self.warnings += 1
        print(issue)
    
    def check_line_length(self, file_path, lines):
        """Check line length (max 120 characters)"""
        for i, line in enumerate(lines, 1):
            if len(line.rstrip()) > 120:
                self.log_issue(file_path, i, "warning", "line_length", 
                             f"Line should be 120 characters or less: currently {len(line.rstrip())} characters")
    
    def check_force_unwrapping(self, file_path, lines):
        """Check for force unwrapping (!), excluding comments"""
        for i, line in enumerate(lines, 1):
            # Skip comments
            if "//" in line:
                code_part = line.split("//")[0]
            else:
                code_part = line
            
            # Look for ! that's not part of != or other operators
            if re.search(r'[^!=<>]\s*!\s*[^=]', code_part):
                self.log_issue(file_path, i, "warning", "force_unwrapping", 
                             "Avoid force unwrapping! Use optional binding or guard statements")
    
    def check_try_force(self, file_path, lines):
        """Check for try! usage"""
        for i, line in enumerate(lines, 1):
            if "try!" in line and not line.strip().startswith("//"):
                self.log_issue(file_path, i, "warning", "try_force", 
                             "Avoid try! - use proper error handling with do-catch or try?")
    
    def check_function_length(self, file_path, lines):
        """Check function body length (max 60 lines)"""
        in_function = False
        function_start = 0
        brace_count = 0
        
        for i, line in enumerate(lines, 1):
            stripped = line.strip()
            
            # Detect function start
            if re.match(r'\s*(private\s+|public\s+|internal\s+|fileprivate\s+)*func\s+', stripped):
                if '{' in line:
                    in_function = True
                    function_start = i
                    brace_count = line.count('{') - line.count('}')
            elif in_function:
                brace_count += line.count('{') - line.count('}')
                
                # Function ended
                if brace_count <= 0:
                    function_length = i - function_start + 1
                    if function_length > 60:
                        self.log_issue(file_path, function_start, "warning", "function_body_length",
                                     f"Function body should be 60 lines or less: currently {function_length} lines")
                    in_function = False
    
    def check_todo_format(self, file_path, lines):
        """Check TODO comment format"""
        for i, line in enumerate(lines, 1):
            if "// TODO" in line and not re.search(r'// TODO:', line):
                self.log_issue(file_path, i, "warning", "todo_format", 
                             "TODO comments should include context: // TODO: Description")
    
    def check_naming_conventions(self, file_path, lines):
        """Check basic naming conventions"""
        for i, line in enumerate(lines, 1):
            # Check for camelCase variables/functions
            if re.search(r'\b(let|var)\s+([A-Z][a-zA-Z0-9]*)\s*=', line):
                self.log_issue(file_path, i, "warning", "variable_name", 
                             "Variable names should start with lowercase letter (camelCase)")
            
            # Check for PascalCase types
            if re.search(r'\b(class|struct|enum|protocol)\s+([a-z][a-zA-Z0-9]*)', line):
                self.log_issue(file_path, i, "warning", "type_name", 
                             "Type names should start with uppercase letter (PascalCase)")
    
    def check_file(self, file_path):
        """Check a single Swift file"""
        try:
            with open(file_path, 'r', encoding='utf-8') as f:
                lines = f.readlines()
            
            print(f"Linting {file_path}")
            
            # Run all checks
            self.check_line_length(file_path, lines)
            self.check_force_unwrapping(file_path, lines)
            self.check_try_force(file_path, lines)
            self.check_function_length(file_path, lines)
            self.check_todo_format(file_path, lines)
            self.check_naming_conventions(file_path, lines)
            
        except Exception as e:
            print(f"Error checking {file_path}: {e}")
    
    def check_directory(self, directory):
        """Check all Swift files in a directory"""
        swift_files = glob.glob(os.path.join(directory, "**/*.swift"), recursive=True)
        
        if not swift_files:
            print(f"No Swift files found in {directory}")
            return
        
        print(f"Found {len(swift_files)} Swift files to check")
        
        for file_path in swift_files:
            self.check_file(file_path)
    
    def summary(self):
        """Print summary of issues found"""
        if self.warnings == 0 and self.errors == 0:
            print("✅ No issues found!")
        else:
            print(f"\n📊 Summary: {self.warnings} warnings, {self.errors} errors")
        
        return self.errors > 0

def main():
    if len(sys.argv) < 2:
        print("Usage: python3 swift-check.py <directory>")
        sys.exit(1)
    
    directory = sys.argv[1]
    if not os.path.isdir(directory):
        print(f"Error: {directory} is not a directory")
        sys.exit(1)
    
    checker = SwiftChecker()
    checker.check_directory(directory)
    has_errors = checker.summary()
    
    sys.exit(1 if has_errors else 0)

if __name__ == "__main__":
    main()