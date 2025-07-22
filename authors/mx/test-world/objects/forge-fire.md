# Forge Fire

An intense fire that burns constantly in the castle forge, fed by coal and maintained at the perfect temperature for metalworking. The flames dance with blues and whites at their hottest points, capable of melting even the hardest steel. The heat radiates outward in waves, making the entire forge uncomfortably warm.

---
object_id: forge-fire
type: environmental
category: tool
weight: immovable
value: 0
properties:
  material: fire
  temperature: extreme
  dangerous: true
  persistent: true
actions:
  - heat-metal: "The metal glows red-hot in the flames"
  - examine: "The fire burns with incredible intensity"
  - approach: "The heat is overwhelming up close"
effects:
  - provides-heat: true
  - enables-smithing: true
  - light-source: true
---