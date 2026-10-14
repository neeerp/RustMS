#include "UIPartyHP.h"

#include "../../Constants.h"
#include "../../Gameplay/Stage.h"
#include "../../Util/NxFiles.h"

#include <nlnx/nx.hpp>

namespace ms
{
	UIPartyHP::UIPartyHP(const Party& party, const CharStats& stats) :
		UIElement(Point<int16_t>(), Point<int16_t>(WIDTH, 0)), party(party), stats(stats)
	{
		nl::node src = NxFiles::ui_classic["UIWindow.img"]["UserList"]["Party"]["PartyHP"];

		nw = src["nw"];
		ne = src["ne"];
		w = src["w"];
		e = src["e"];
		sw = src["sw"];
		se = src["se"];
		graduation = src["GaugeBar"]["graduation"];
		bar = src["GaugeBar"]["bar"];
		gauge = src["GaugeBar"]["gauge"];

		update_screen(Constants::Constants::get().get_viewwidth(), Constants::Constants::get().get_viewheight());
	}

	void UIPartyHP::draw(float) const
	{
		if (rows.empty())
			return;

		// The pieces' origins hang the borders around the content.
		Point<int16_t> content = position + Point<int16_t>(BORDER, EDGE);
		int16_t height = height_for(party) - 2 * EDGE;
		int16_t inner = WIDTH - BORDER;

		nw.draw(content);
		ne.draw(content);
		w.draw(DrawArgument(content, Point<int16_t>(BORDER, height)));
		e.draw(DrawArgument(content, Point<int16_t>(inner, height)));
		sw.draw(content + Point<int16_t>(0, height));
		se.draw(content + Point<int16_t>(0, height));

		for (size_t i = 0; i < rows.size(); i++)
		{
			const Row& row = rows[i];
			Point<int16_t> at = content + Point<int16_t>(GAUGE_X, GAUGE_Y + static_cast<int16_t>(i) * ROW);

			row.label.draw(content + Point<int16_t>(0, NAME_Y + static_cast<int16_t>(i) * ROW));
			bar.draw(at);

			int32_t hp = 0, maxhp = 0;

			if (row.id == Stage::get().get_player().get_oid())
			{
				hp = stats.get_stat(MapleStat::Id::HP);
				maxhp = stats.get_total(EquipStat::Id::HP);
			}
			else if (row.nearby)
			{
				if (auto member = party.find(row.id))
				{
					hp = member->hp;
					maxhp = member->maxhp;
				}
			}

			if (maxhp > 0 && hp > 0)
			{
				auto fill = static_cast<int16_t>(GAUGE_WIDTH * static_cast<int64_t>(std::min(hp, maxhp)) / maxhp);
				gauge.draw(DrawArgument(at + Point<int16_t>(3, 3), Point<int16_t>(std::max<int16_t>(fill, 1), 0)));
			}

			graduation.draw(at);
		}
	}

	void UIPartyHP::update()
	{
		int32_t self = Stage::get().get_player().get_oid();
		int32_t mapid = Stage::get().get_mapid();
		const auto& members = party.get_members();
		bool changed = members.size() != rows.size();

		for (size_t i = 0; !changed && i < members.size(); i++)
		{
			const PartyMember& member = members[i];
			bool nearby = member.id == self || (member.online() && member.map_id == mapid);
			changed = rows[i].id != member.id || rows[i].name != member.name || rows[i].nearby != nearby;
		}

		if (!changed)
			return;

		rows.clear();

		for (const PartyMember& member : members)
		{
			bool nearby = member.id == self || (member.online() && member.map_id == mapid);
			Color::Name color = nearby ? Color::Name::MINESHAFT : Color::Name::DUSTYGRAY;
			rows.push_back({ member.id, member.name, nearby, Text(Text::A12M, Text::LEFT, color, member.name) });
		}

		dimension = Point<int16_t>(WIDTH, height_for(party));
	}

	void UIPartyHP::update_screen(int16_t new_width, int16_t)
	{
		position = Point<int16_t>(new_width - RIGHT_MARGIN - WIDTH, 0);
	}

	bool UIPartyHP::is_in_range(Point<int16_t>) const
	{
		// Clicks fall through to the map.
		return false;
	}

	UIElement::Type UIPartyHP::get_type() const
	{
		return TYPE;
	}

	int16_t UIPartyHP::height_for(const Party& party)
	{
		auto count = static_cast<int16_t>(party.get_members().size());

		if (count == 0)
			return 0;

		// Two rows make a 37 px box in footage.
		return 2 * EDGE + count * ROW - 11;
	}
}
