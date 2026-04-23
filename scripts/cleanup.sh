#!/bin/bash

# Monthly Data Cleanup Script for Project Mirror
# This script should be run monthly via cron or GitHub Actions

set -e  # Exit on error

# Configuration
BACKEND_URL="${BACKEND_URL:-http://localhost:8080}"
USER_ID="${USER_ID:-default_user}"
DAYS_THRESHOLD="${DAYS_THRESHOLD:-270}"
MIN_DELETION_SCORE="${MIN_DELETION_SCORE:-40.0}"
LIMIT="${LIMIT:-100}"

echo "Starting monthly data cleanup..."
echo "Backend URL: $BACKEND_URL"
echo "User ID: $USER_ID"
echo "Days Threshold: $DAYS_THRESHOLD"
echo "Min Deletion Score: $MIN_DELETION_SCORE"
echo "Limit: $LIMIT"

# Make the API call
response=$(curl -s -w "\n%{http_code}" -X POST "$BACKEND_URL/api/v1/maintenance/cleanup" \
  -H "Content-Type: application/json" \
  -d "{
    \"user_id\": \"$USER_ID\",
    \"days_threshold\": $DAYS_THRESHOLD,
    \"min_deletion_score\": $MIN_DELETION_SCORE,
    \"limit\": $LIMIT
  }")

# Extract HTTP status code (last line)
http_code=$(echo "$response" | tail -n1)

# Extract response body (all lines except last)
body=$(echo "$response" | sed '$d')

echo "HTTP Status: $http_code"
echo "Response: $body"

# Check if successful
if [ "$http_code" -eq 200 ]; then
  echo "✅ Cleanup completed successfully"
  
  # Parse deleted count (requires jq)
  if command -v jq &> /dev/null; then
    deleted_count=$(echo "$body" | jq -r '.deleted_count')
    echo "📊 Deleted episodes: $deleted_count"
  fi
  
  exit 0
else
  echo "❌ Cleanup failed with status $http_code"
  exit 1
fi
