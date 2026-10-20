#pragma once

#include "Tooltip.h"
#include "TooltipBox.h"

#include "../../Character/SkillMacros.h"
#include "../../Graphics/Text.h"

#include <array>
#include <string>

namespace ms
{
	// A skill macro's name, what macros are for, its icon and its skills in
	// red (v83 FUN_008f2276, report 206b), over a macro's icon in the macro
	// window and over a key bound to a macro.
	class MacroTooltip : public Tooltip
	{
	public:
		MacroTooltip();

		void draw(Point<int16_t> position) const override;

		// The macro at `index`. Past the list it has no name or skills, and
		// past the icons no icon, as in v83.
		void set_macro(size_t index, const SkillMacros &macros);

	private:
		size_t index;
		std::string name;
		std::array<int32_t, 3> skills;

		TooltipBox box;
		TooltipIcon icon;
		std::array<Texture, SkillMacros::MAX_MACROS> macro_icons;

		Text title;
		Text desc;
		Text skill_names;
	};
}
