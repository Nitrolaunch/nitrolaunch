echo "Rust files:"
find src crates gui plugins -name "*.rs" | xargs wc -l
echo "TSX files:"
find gui/src -name "*.tsx" | xargs wc -l
echo "CSS files:"
find gui/src -name "*.css" | xargs wc -l