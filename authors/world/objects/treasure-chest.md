# Treasure Chest

An ornate wooden chest bound with iron and secured with multiple locks. The wood is polished to a high sheen, and precious metals inlay complex patterns across its surface. Despite its beauty, the chest is built like a fortress, designed to protect whatever treasures lie within.

---
object_id: treasure-chest
type: container
category: storage
weight: heavy
value: 500
properties:
  material: wood-iron
  locked: true
  trapped: possibly
  ornate: true
actions:
  - examine: "The craftsmanship rivals the finest furniture"
  - attempt-open: "The locks hold firm against your efforts"
  - listen: "You hear the faint sound of coins shifting inside"
contents:
  - gold-coins
  - precious-gems
  - royal-documents
  - silver-chalice
security:
  - multiple-locks: true
  - trap-detection: unknown
---