#include "MapWedding.h"

#include "../Stage.h"

#include "../../Audio/Audio.h"
#include "../../IO/UI.h"
#include "../../IO/UITypes/UINpcTalk.h"
#include "../../Net/Packets/MarriagePackets.h"

#include <nlnx/nx.hpp>

namespace ms
{
	namespace
	{
		// Every page gets its own number, across maps.
		uint32_t next_page = 0;

		// The bless question (pool 0x106c).
		constexpr const char *BLESS_TEXT = "Do you want to bless this couple?";
	}

	MapWedding::MapWedding(int32_t mapid) : mapid(mapid), priest(wedding::priest_of(mapid)) {}

	MapWedding::MapWedding() {}

	void MapWedding::progress(uint8_t new_step, int32_t new_groom, int32_t new_bride, int32_t self)
	{
		if (!priest)
			return;

		groom = new_groom;
		bride = new_bride;
		couple = self == groom || self == bride;

		// The open page ends, as No for a guest's bless question and NEXT
		// for any other (0x5815c1).
		bool ending = close_page();

		step = new_step;

		if (step == 0)
		{
			hearts = false;
			Music(wedding::bgm_of(mapid)).play();
		}

		wedding::Page shown = wedding::page_for(mapid, step, couple);

		if (shown == wedding::Page::NONE)
		{
			// Nothing replaces the page ended, so it returns now.
			if (ending)
				settle();

			return;
		}

		UI::get().emplace<UINpcTalk>();
		UI::get().enable();

		auto talk = UI::get().get_element<UINpcTalk>();

		if (!talk)
			return;

		page = ++next_page;
		pages.show();

		if (shown == wedding::Page::BLESS)
		{
			talk->change_text(priest, UINpcTalk::SENDYESNO, 0, 0, BLESS_TEXT);
		}
		else
		{
			std::string line = nl::nx::string["Npc.img"][std::to_string(priest)][wedding::line_of(step)];
			talk->change_text(priest, 0, 0x100, 0, line);
		}

		talk->set_local_responder([answered = page](int8_t response, int32_t)
		{
			Stage::get().answer_priest(answered, response);
		}, page);
	}

	void MapWedding::show_hearts()
	{
		if (!priest)
			return;

		if (close_page())
			settle();

		hearts = true;
		effect = nl::nx::effect["BasicEff.img"]["Wedding"];
	}

	void MapWedding::answered(uint32_t answered_page, int8_t response)
	{
		if (answered_page != page || !pages.open())
			return;

		// Only a guest's Yes blesses the couple.
		if (wedding::page_for(mapid, step, couple) == wedding::Page::BLESS && response == 1)
			BlessCouplePacket().dispatch();

		settle();
	}

	int32_t MapWedding::get_groom() const
	{
		return groom;
	}

	int32_t MapWedding::get_bride() const
	{
		return bride;
	}

	void MapWedding::update(std::optional<Point<int16_t>> groompos, std::optional<Point<int16_t>> bridepos)
	{
		if (!hearts)
			return;

		effect.update();

		if (groompos && bridepos)
		{
			auto [x, y] = wedding::hearts_at(groompos->x(), groompos->y(), bridepos->x(), bridepos->y());
			hearts_pos = Point<int16_t>(static_cast<int16_t>(x), static_cast<int16_t>(y));
		}
		else
		{
			hearts_pos.reset();
		}
	}

	void MapWedding::draw(Point<int16_t> viewpos, float alpha) const
	{
		if (hearts && hearts_pos)
			effect.draw(DrawArgument(*hearts_pos + viewpos), alpha);
	}

	bool MapWedding::close_page()
	{
		if (!pages.open())
			return false;

		auto talk = UI::get().get_element<UINpcTalk>();

		if (!talk || !talk->is_active() || talk->get_local_tag() != page)
		{
			pages.forget();

			return false;
		}

		talk->set_local_responder(nullptr);
		talk->deactivate();

		return true;
	}

	void MapWedding::settle()
	{
		for (uint8_t talks = pages.settle(mapid, step, couple); talks > 0; talks--)
			WeddingTalkPacket(step).dispatch();
	}
}
