#!/bin/bash

# Ralph Wiggum Loop for Amp

MAX_ITERATIONS=${1:-10}

echo "🔁 Starting Ralph loop (max $MAX_ITERATIONS iterations)..."
echo "   Press Ctrl+C to stop"
echo ""

for i in $(seq 1 "$MAX_ITERATIONS"); do
  echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
  echo "📍 Iteration $i of $MAX_ITERATIONS"
  echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

  cat LOOP.md | amp --dangerously-allow-all -x

  # Check for completion signal
  if grep -q "PROJECT COMPLETE" CHANGELOG.md 2>/dev/null; then
    echo ""
    echo "✅ Project complete! Stopping loop."
    exit 0
  fi
done

echo ""
echo "🏁 Reached max iterations ($MAX_ITERATIONS)."
