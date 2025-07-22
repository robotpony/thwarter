# Flowing Water

Crystal-clear water flows continuously through the forest stream, creating a soothing babbling sound as it passes over smooth stones. The water is clean and cold, perfect for drinking or washing. Small fish dart through the current, and water plants sway gently in the flow.

---
object_id: flowing-water
type: environmental
category: natural-resource
weight: liquid
value: 0
properties:
  material: water
  temperature: cold
  potable: true
  moving: true
actions:
  - drink: "The cold water refreshes you completely"
  - examine: "The water is crystal clear and clean"
  - touch: "The water is shockingly cold"
effects:
  - restores-thirst: true
  - enables-washing: true
  - attracts-wildlife: true
---