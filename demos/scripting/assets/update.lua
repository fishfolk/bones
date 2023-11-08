local entities = world.resources:get(s"Entities")
local entity = entities:create()
info(entity)
entities:kill(entity)
