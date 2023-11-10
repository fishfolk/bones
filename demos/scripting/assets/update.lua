local Vec3 = s"Vec3"
local Transform = s"Transform"
local entities = world.resources:get(s"Entities")
local components = world.components

local entity = entities:create();
local t = Transform:create()

t.scale.y = 7
components:insert(entity, t)

local comp = components:get(entity, Transform);

components:remove(entity, Transform);

info(components:get(entity, Transform));
entities:kill(entity)
