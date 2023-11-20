local Vec3 = s"Vec3"
local Transform = s"Transform"
local Sprite = s"Sprite"
local Time = s"Time"
local Entities = s"Entities"

local function startup()
  local meta = assets.root
  local entities = resources:get(Entities)

  local ent = entities:create()
  components:insert(ent, Transform:create())
  local sprite = Sprite:create()
  sprite.image = meta.sprite
  components:insert(ent, sprite)
end

local function update()
  local entities = resources:get(Entities)
  local time = resources:get(Time)

  for ent, t, s in entities:iter_with(Transform, Sprite) do
    t.translation.x = math.sin(time.elapsed_seconds * 2) * 100
    t.translation.y = math.sin(time.elapsed_seconds * 1.8) * 100
  end
end

session:add_startup_system(startup)
session:add_system_to_stage(CoreStage.Update, update)
