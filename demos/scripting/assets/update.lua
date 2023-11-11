local Transform = s"Transform"
local Entity = s"Entity"
local Time = s"Time"
local Entities = s"Entities"

local components = world.components
local entities = world.resources:get(Entities)

local time = world.resources:get(Time)

local ent = Entity:create()
ent[0] = 0
ent[1] = 0

local t = components:get(ent, Transform)
t.translation.x = math.sin(time.elapsed_seconds * 2) * 100
t.translation.y = math.sin(time.elapsed_seconds * 1.8) * 200

-- t.scale.y = 7
-- components:insert(entity, t)

-- local comp = components:get(entity, Transform);

-- components:remove(entity, Transform);

-- info(components:get(entity, Transform));
-- entities:kill(entity)
