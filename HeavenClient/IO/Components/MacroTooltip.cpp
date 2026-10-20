#include "MacroTooltip.h"

#include "../../Data/SkillData.h"
#include "../../Util/NxFiles.h"

#include <nlnx/nx.hpp>

namespace ms
{
	namespace
	{
		// v83 sizes the box 290 by 142 whatever it holds. Text tops are 3
		// and shapes 1 above v83's, as in the skill tooltip.
		const Point<int16_t> SIZE = { 290, 142 };
		const Point<int16_t> TITLE_POS = { 145, 5 };
		const Point<int16_t> DESC_POS = { 15, 24 };
		constexpr int16_t DESC_WIDTH = 252;
		const Point<int16_t> ICON_POS = { 10, 56 };
		const Point<int16_t> SKILLS_POS = { 92, 54 };
		constexpr int16_t SKILLS_WIDTH = 175;

		// StringPool 0x10f0.
		constexpr const char *DESCRIPTION = "Register the frequently-used skills for a quick access with a single button.";
	}

	MacroTooltip::MacroTooltip() :
		index(SkillMacros::MAX_MACROS), skills{},
		desc(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::WHITE, DESCRIPTION, DESC_WIDTH, true,
			 tooltip::LINE_ADJ)
	{
		nl::node src = NxFiles::ui_classic["UIWindow.img"]["SkillMacro"]["Macroicon"];

		for (size_t i = 0; i < SkillMacros::MAX_MACROS; i++)
			macro_icons[i] = src[std::to_string(i)]["icon"];
	}

	void MacroTooltip::set_macro(size_t i, const SkillMacros &macros)
	{
		std::array<int32_t, 3> ids = { macros.skill(i, 0), macros.skill(i, 1), macros.skill(i, 2) };

		if (index == i && name == macros.name(i) && skills == ids)
			return;

		index = i;
		name = macros.name(i);
		skills = ids;

		std::string list;

		for (int32_t id : skills)
		{
			if (!id)
				continue;

			if (!list.empty())
				list += "\\n";

			list += SkillData::get(id).get_name();
		}

		title = Text(Text::Font::A12B, Text::Alignment::CENTER, Color::Name::WHITE, name);
		skill_names = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::TIPRED, list, SKILLS_WIDTH, true,
						   tooltip::LINE_ADJ);
	}

	void MacroTooltip::draw(Point<int16_t> position) const
	{
		Point<int16_t> pos = tooltip::place(position, SIZE);

		box.draw(pos, SIZE);
		title.draw(pos + TITLE_POS);
		desc.draw(pos + DESC_POS);

		if (index < macro_icons.size())
			icon.draw(macro_icons[index], pos + ICON_POS);

		skill_names.draw(pos + SKILLS_POS);
	}
}
