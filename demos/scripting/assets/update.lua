local Transform = s"Transform"
local Sprite = s"Sprite"
local Entity = s"Entity"
local Time = s"Time"
local Entities = s"Entities"

local components = world.components
local entities = world.resources:get(Entities)

local time = world.resources:get(Time)

for ent, t, s in entities:iter_with(Transform, Sprite) do
  t.translation.x = math.sin(time.elapsed_seconds * 2) * 100
  t.translation.y = math.sin(time.elapsed_seconds * 1.8) * 200
end
