#include "DirectionScene.h"

#include "Stage.h"

#include "../Constants.h"
#include "../Audio/Audio.h"

#include <algorithm>

namespace ms
{
	void DirectionScene::play(nl::node part, Point<int16_t> position)
	{
		auto or_default = [](nl::node value, int16_t at) { return value ? static_cast<int16_t>(value) : at; };

		for (nl::node src : part)
		{
			direction::Event event;
			event.type = src["type"];
			event.start = now + static_cast<int32_t>(src["start"]);
			event.visual = src["visual"].get_string();
			event.sound = src["sound"].get_string();
			event.field = src["field"];
			event.x = or_default(src["x"], position.x()) + static_cast<int16_t>(src["dx"]);
			event.y = or_default(src["y"], position.y()) + static_cast<int16_t>(src["dy"]);
			event.x1 = src["x1"];
			event.y1 = src["y1"];
			event.duration = src["duration"];
			event.z = src["z"];
			waiting.push_back(event);
		}
	}

	std::optional<int32_t> DirectionScene::update()
	{
		now += Constants::TIMESTEP;

		std::optional<int32_t> field;

		for (auto it = waiting.begin(); it != waiting.end();)
		{
			if (it->start > now)
			{
				++it;
				continue;
			}

			if (it->type == direction::Event::FIELD)
				field = it->field;

			// The picture is the visual's first child.
			if (it->type == direction::Event::VISUAL)
				if (nl::node src = Stage::effect_node(it->visual)["0"])
					showing.push_back({ *it, Animation(src), it->start });

			if ((it->type == direction::Event::VISUAL || it->type == direction::Event::SOUND) && !it->sound.empty())
				if (nl::node src = Stage::effect_node(it->sound))
					Sound(src).play(100);

			it = waiting.erase(it);
		}

		// Those with equal z keep the order they came in.
		std::stable_sort(showing.begin(), showing.end(),
						 [](const Visual &a, const Visual &b) { return a.event.z < b.event.z; });

		// Each plays once, then is gone.
		showing.erase(std::remove_if(showing.begin(), showing.end(),
									 [](Visual &visual) { return visual.animation.update(); }),
					  showing.end());

		return field;
	}

	void DirectionScene::draw(Point<int16_t> viewpos, float alpha) const
	{
		for (const Visual &visual : showing)
		{
			direction::Position at = direction::position(visual.event, now - visual.began);
			visual.animation.draw(DrawArgument(viewpos + Point<int16_t>(at.x, at.y)), alpha);
		}
	}

	void DirectionScene::clear()
	{
		now = 0;
		waiting.clear();
		showing.clear();
	}
}
