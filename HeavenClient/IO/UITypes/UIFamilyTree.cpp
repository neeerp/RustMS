#include "UIFamilyTree.h"

#include "../FamilyActions.h"
#include "../UI.h"

#include "../Components/MapleButton.h"

#include "../../Character/Job.h"
#include "../../Gameplay/Stage.h"
#include "../../Util/Misc.h"
#include "../../Util/NxFiles.h"

#ifdef USE_NX
#include <nlnx/nx.hpp>
#endif

namespace ms
{
	namespace
	{
		// Window-local, from b044. As in the Family window, text goes up
		// by the difference between where v83 and we draw a line's top.
		constexpr int16_t TEXT_RAISE = 6;
		const Point<int16_t> POSITION(111, 107);
		const Point<int16_t> DRAG_AREA(578, 25);

		const Point<int16_t> JUNIOR_AT(21, 57), BYE_AT(71, 57);

		// The boxes (0xbe2778): the leader, two seniors, the one viewed and
		// beside them their senior's other junior, then two generations of
		// juniors.
		const std::array<Point<int16_t>, FamilyChart::SLOTS> SLOT_AT = {
			Point<int16_t>(222, 40), Point<int16_t>(222, 94), Point<int16_t>(222, 146),
			Point<int16_t>(222, 198), Point<int16_t>(365, 198),
			Point<int16_t>(80, 250), Point<int16_t>(365, 250),
			Point<int16_t>(13, 302), Point<int16_t>(148, 302), Point<int16_t>(297, 302), Point<int16_t>(432, 302)
		};
		constexpr int16_t SLOT_W = 133, SLOT_H = 34;
		// The level line is wider than its box and starts left of it.
		constexpr int16_t LEVEL_X = -10, LEVEL_W = 153;
		constexpr int16_t GLOW_MARGIN = 3;

		constexpr int16_t TITLE_X = 205, TITLE_Y = 10, TITLE_W = 167;
		constexpr int16_t TOTAL_X = 29, TOTAL_Y = 41, TOTAL_W = 72;
		const Point<int16_t> SENIORS_AT(295, 80);
		const Rectangle<int16_t> TOTAL_TIP(19, 120, 20, 53);

		// StringPool 0x11f1, 0x11ef, 0x11ec, 0x11f0, 0x11f2, 0x11f3.
		constexpr const char* NO_FAMILY = "(You do not have a family yet.)";
		constexpr const char* NO_MEMBERS = "(You do not have family members.)";
		constexpr const char* ADD_JUNIOR = "[Please add a Junior.]";

		// 0x1215-0x1218.
		constexpr const char* JUNIOR_HELP = "If the Junior slot is empty, then you can add a Junior that will help enhance your Rep.";
		constexpr const char* SENIOR_BYE_HELP = "Your Senior will sever ties and start a new family.\\nThe members below you will belong to your family.";
		constexpr const char* JUNIOR_BYE_HELP = "This will end your relationship with your Junior.\\nThe Junior and everyone underneath will all be removed from the Family.";
		constexpr const char* TOTAL_HELP = "This covers everyone in the Family.\\nThe number may be higher than the one shown in the Family Tree.";

		// 0x121b-0x1225, as the member tooltips build them (0x80bb81,
		// 0x80bf45). v83 indents the uses with a tab.
		constexpr const char* PASSED_SENIOR = "#c* You have surpassed your Senior's level, so your cumulative Rep#\\n#cwill be reduced in half. Please help your Senior level up!#\\n";
		constexpr const char* PASSED_YOU = "#c* Your Junior has surpassed your level, so your cumulative Rep#\\n#cwill be reduced in half. Please try harder to level up!#\\n";
		constexpr const char* IN_USE = "Entitlement in use :\\n";
		constexpr const char* USED_TODAY = "Entitlement used today :\\n";
		constexpr const char* SEPARATOR = "______________________________________________\\n";
		// 0x2ce: where a member is with no map.
		constexpr const char* CASH_SHOP = "Cash Shop";

		std::string number(int32_t value)
		{
			std::string text = std::to_string(value);
			string_format::split_number(text);

			return text;
		}

		// Centred in a box `width` wide. v83 halves the room left over as
		// unsigned (0x808212), so a text wider than its box lands far off
		// screen and is never seen.
		Text centred(Text::Font font, Color::Name color, const std::string& text = "", uint16_t width = SLOT_W)
		{
			Text line(font, Text::Alignment::CENTER, color, text, 0, false);

			return line.width() > width ? Text() : line;
		}

		bool inside(const Rectangle<int16_t>& rect, Point<int16_t> local)
		{
			return local.x() >= rect.left() && local.x() < rect.right()
				&& local.y() >= rect.top() && local.y() < rect.bottom();
		}
	}

	UIFamilyTree::UIFamilyTree() : UIDragElement<PosFAMILYTREE>(DRAG_AREA)
	{
		// v83 opens it in the same place each time.
		position = POSITION;

		nl::node src = NxFiles::ui_classic["UIWindow.img"]["FamilyTree"];

		background = src["backgrnd"];
		selected_glow = src["selected"];
		dimension = background.get_dimensions();

		for (size_t i = 0; i < 2; i++)
		{
			leader_plate[i] = src["PlateLeader"][std::to_string(i)];
			other_plate[i] = src["PlateOthers"][std::to_string(i)];
		}

		buttons[JUNIOR] = std::make_unique<MapleButton>(src["BtJuniorEntry"], JUNIOR_AT);
		buttons[BYE] = std::make_unique<MapleButton>(src["BtBye"], BYE_AT);

		lay_out();
	}

	void UIFamilyTree::set_chart(FamilyChart value)
	{
		chart = std::move(value);
		selected = -1;
		hovered = -1;

		lay_out();
	}

	void UIFamilyTree::set_online(const std::string& name, bool online)
	{
		for (FamilyNode& node : chart.nodes)
			if (node.name == name)
				node.online = online;
	}

	bool UIFamilyTree::is_mine() const
	{
		return chart.mine;
	}

	void UIFamilyTree::deselect()
	{
		selected = -1;
	}

	void UIFamilyTree::bye()
	{
		if (!chart.mine)
			return;

		if (selected < 0)
			return FamilyActions::prompt_parting(chart);

		if (auto parting = chart.parting_at(static_cast<size_t>(selected)))
			FamilyActions::confirm_parting(*parting);
	}

	void UIFamilyTree::lay_out()
	{
		const FamilyNode* leader = chart.at(FamilyChart::LEADER);

		title = centred(Text::Font::A12M, Color::Name::WHITE, leader ? leader->name + " Family" : NO_FAMILY, TITLE_W);
		total = Text(Text::Font::A12M, Text::Alignment::RIGHT, Color::Name::BLACK, leader ? number(chart.count(-1)) : "0", TOTAL_W);

		int32_t above = chart.count(0);

		if (chart.at(1))
			above--;

		if (chart.at(FamilyChart::SENIOR))
			above--;

		seniors = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::WHITE, "Senior(" + std::to_string(above) + " ppl.)");

		for (size_t slot = 0; slot < FamilyChart::SLOTS; slot++)
		{
			const FamilyNode* node = chart.at(slot);

			names[slot] = Text();
			levels[slot] = Text();
			empties[slot] = Text();
			juniors[slot] = Text();

			if (node)
			{
				Color::Name color = chart.passed_senior(slot) ? Color::Name::BRIGHTRED : Color::Name::WHITE;

				names[slot] = centred(Text::Font::A12M, color, node->name);
				levels[slot] = centred(Text::Font::A11M, Color::Name::WHITE,
					"Lv." + std::to_string(node->level) + " " + Job::get_name(node->job), LEVEL_W);
			}
			else if (slot == 5 || slot == 6)
			{
				empties[slot] = centred(Text::Font::A12M, Color::Name::FAMILYBLUE, ADD_JUNIOR);
			}
			else if (slot != FamilyChart::OWNER)
			{
				// Wider than the box, so blank as in v83 (see centred).
				empties[slot] = centred(Text::Font::A12M, Color::Name::SILVER, NO_MEMBERS);
			}

			// Shown for an empty box too, with none.
			if (slot >= 7)
			{
				int32_t below = node ? chart.count(node->id) : 0;
				juniors[slot] = centred(Text::Font::A12M, Color::Name::WHITE, "Junior(" + std::to_string(below) + " ppl.)");
			}
		}

		buttons[JUNIOR]->set_state(chart.can_add_junior() ? Button::State::NORMAL : Button::State::DISABLED);
		// On whatever is selected, in the player's own tree only.
		buttons[BYE]->set_state(chart.mine ? Button::State::NORMAL : Button::State::DISABLED);
	}

	void UIFamilyTree::draw(float inter) const
	{
		background.draw(position);

		Point<int16_t> raise(0, TEXT_RAISE);

		for (size_t slot = 0; slot < FamilyChart::SLOTS; slot++)
		{
			Point<int16_t> at = position + SLOT_AT[slot];
			const FamilyNode* node = chart.at(slot);
			Point<int16_t> centre(SLOT_W / 2, 0);

			if (node)
			{
				// The one viewed sits in a box drawn on the background.
				if (slot == FamilyChart::LEADER)
					leader_plate[node->online].draw(at);
				else if (slot != FamilyChart::OWNER)
					other_plate[node->online].draw(at);

				int16_t lift = slot == FamilyChart::LEADER ? 1 : 0;

				names[slot].draw(at + centre + Point<int16_t>(0, 5 + lift) - raise);
				levels[slot].draw(at + Point<int16_t>(LEVEL_X + LEVEL_W / 2, 20 + lift) - raise);
			}
			else
			{
				empties[slot].draw(at + centre + Point<int16_t>(0, 11) - raise);
			}

			if (slot >= 7)
				juniors[slot].draw(at + centre + Point<int16_t>(0, 58) - raise);

			// The glow's origin puts its corner three pixels out.
			if (static_cast<int16_t>(slot) == selected || static_cast<int16_t>(slot) == hovered)
				selected_glow.draw(at);
		}

		title.draw(position + Point<int16_t>(TITLE_X + TITLE_W / 2, TITLE_Y) - raise);
		total.draw(position + Point<int16_t>(TOTAL_X + TOTAL_W, TOTAL_Y) - raise);
		seniors.draw(position + SENIORS_AT - raise);

		UIElement::draw(inter);
	}

	int16_t UIFamilyTree::slot_at(Point<int16_t> local) const
	{
		for (size_t slot = 0; slot < FamilyChart::SLOTS; slot++)
		{
			Point<int16_t> at = SLOT_AT[slot];

			if (inside(Rectangle<int16_t>(at.x(), at.x() + SLOT_W, at.y(), at.y() + SLOT_H), local))
				return static_cast<int16_t>(slot);
		}

		return -1;
	}

	bool UIFamilyTree::hoverable(int16_t slot) const
	{
		if (slot == FamilyChart::SENIOR)
			return chart.at(FamilyChart::SENIOR) != nullptr;

		return slot == 5 || slot == 6;
	}

	Cursor::State UIFamilyTree::send_cursor(bool clicked, Point<int16_t> cursorpos)
	{
		Cursor::State state = UIDragElement::send_cursor(clicked, cursorpos);

		if (dragged || !chart.mine)
		{
			hovered = -1;
			clear_tooltip();

			return state;
		}

		Point<int16_t> local = cursorpos - position;
		int16_t slot = slot_at(local);

		hovered = hoverable(slot) ? slot : -1;

		if (clicked && !is_holding_button())
		{
			if (hoverable(slot))
			{
				selected = chart.at(slot) ? slot : -1;

				if (!chart.at(slot))
					FamilyActions::prompt_junior();
			}
			else if (slot >= 0)
			{
				selected = -1;
			}
		}

		std::string text;

		if (buttons[JUNIOR]->get_state() == Button::State::MOUSEOVER)
			text = JUNIOR_HELP;
		else if (buttons[BYE]->get_state() == Button::State::MOUSEOVER)
			text = selected == FamilyChart::SENIOR ? SENIOR_BYE_HELP : JUNIOR_BYE_HELP;
		else if (inside(TOTAL_TIP, local))
			text = TOTAL_HELP;
		else if (slot >= 0 && chart.at(slot))
			text = member_tooltip(slot);

		if (text.empty())
		{
			clear_tooltip();
		}
		else
		{
			UI::get().show_text(Tooltip::Parent::FAMILYTREE, text);
			tooltip = true;
		}

		return state;
	}

	std::string UIFamilyTree::member_tooltip(size_t slot) const
	{
		const FamilyNode& node = *chart.at(slot);
		const FamilyNode* senior = node.senior >= 0 ? &chart.nodes[node.senior] : nullptr;
		const FamilyNode* owner = chart.at(FamilyChart::OWNER);
		std::string text;

		// The player's own box, or the leader's when that is them.
		if (slot == FamilyChart::OWNER || (owner && node.id == owner->id))
		{
			text += "Reps recorded to your Senior : " + number(node.senior_rep) + "\\n";

			if (senior && senior->level < node.level)
				text += PASSED_SENIOR;

			text += SEPARATOR;

			const Family& family = Stage::get().get_player().get_family();
			const std::vector<FamilyPrivilege>& privileges = family.get_privileges();

			// The player's own, even in another's tree (0x80bce7).
			if (int32_t index = family.in_use(); index >= 0)
			{
				std::string name = index < static_cast<int32_t>(privileges.size())
					? privileges[index].name : std::to_string(index);

				text += IN_USE;
				text += "    " + name + "\\n\\n";
			}

			if (!chart.used.empty())
			{

				text += USED_TODAY;

				for (auto& [index, times] : chart.used)
				{
					std::string name = index >= 0 && index < static_cast<int32_t>(privileges.size())
						? privileges[index].name : std::to_string(index);

					text += "    " + name + " - " + std::to_string(times) + " times\\n";
				}
			}
		}
		else
		{
			if (slot >= 5)
			{
				int32_t today = slot <= 6 ? node.senior_rep : node.today;
				text += "Reps recorded today : " + number(today) + "\\n";

				if (senior && senior->level < node.level)
					text += PASSED_YOU;

				text += SEPARATOR;
			}

			if (node.online)
			{
				std::string place = node.map_id < 0 ? CASH_SHOP
					: NxHelper::Map::get_map_info_by_id(node.map_id).name;

				text += "Time elapsed since login : " + std::to_string(node.minutes / 60) + "hrs. "
					+ std::to_string(node.minutes % 60) + "min.\\n";
				text += "Current Location : " + place + "\\n";
			}

			text += "Current Reps : " + number(node.rep) + "\\nTotal Reps : " + number(node.total_rep) + "\\n";
		}

		// The last line's break would leave a blank one.
		while (text.size() >= 2 && text.compare(text.size() - 2, 2, "\\n") == 0)
			text.erase(text.size() - 2);

		return text;
	}

	void UIFamilyTree::remove_cursor()
	{
		UIDragElement::remove_cursor();

		hovered = -1;
		clear_tooltip();
	}

	void UIFamilyTree::clear_tooltip()
	{
		if (!tooltip)
			return;

		tooltip = false;
		UI::get().clear_tooltip(Tooltip::Parent::FAMILYTREE);
	}

	void UIFamilyTree::send_key(int32_t, bool pressed, bool escape)
	{
		if (pressed && escape)
		{
			clear_tooltip();
			deactivate();
		}
	}

	// Only a box that lights up, with someone in it, and nothing else.
	void UIFamilyTree::doubleclick(Point<int16_t> cursorpos)
	{
		if (!chart.mine)
			return;

		int16_t slot = slot_at(cursorpos - position);

		if (!hoverable(slot) || !chart.at(slot))
			return;

		selected = slot;
		bye();
	}

	Button::State UIFamilyTree::button_pressed(uint16_t buttonid)
	{
		if (buttonid == JUNIOR)
			FamilyActions::prompt_junior();
		else if (buttonid == BYE)
			bye();

		return Button::State::NORMAL;
	}

	UIElement::Type UIFamilyTree::get_type() const
	{
		return TYPE;
	}
}
