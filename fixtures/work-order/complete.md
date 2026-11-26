```json
{
  "id": "WO-12",
  "title": "Render the work order",
  "state": "ruled",
  "rulings": [
    {
      "words": "Brokkr assigns the work order's id.",
      "date": "2026-11-23",
      "cite": "https://github.com/feedback-loop-ai/brokkr/issues/1272#issuecomment-7097200616"
    }
  ],
  "asks": [
    {
      "number": 1,
      "question": "What assigns the id?",
      "ruling": 1
    },
    {
      "number": 2,
      "question": "Where should this work order live?",
      "ruling": 1
    }
  ],
  "constraints": [
    "Keep the document in the pure core."
  ],
  "caps": {
    "brokkr-core": 60,
    "brokkr-store": 0
  },
  "references": [
    {
      "tracker": "github",
      "item": "feedback-loop-ai/brokkr#1272",
      "link": "https://github.com/feedback-loop-ai/brokkr/issues/1272"
    }
  ],
  "parent": "WO-2",
  "edges": [
    {
      "awaits": "WO-10",
      "condition": "merged"
    },
    {
      "awaits": "WO-8",
      "condition": "delivered-elsewhere"
    }
  ]
}
```

## Intent

Keep the operator's rulings with the work.

The document gives reviews a stable statement of the work.

## Acceptance

- Render an assigned work order byte for byte.
- Read a draft without an id.
