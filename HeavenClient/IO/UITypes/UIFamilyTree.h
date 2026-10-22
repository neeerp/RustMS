#pragma once

#include "../UIDragElement.h"

#include "../../Character/Family.h"
#include "../../Graphics/Text.h"
#include "../../Graphics/Texture.h"

#include <array>

namespace ms
{
	// v83's Family Tree (UIWindow.img/FamilyTree, type 0x1c, report b044):
	// eleven boxes around the character viewed, from the leader at the top
	// to their juniors' juniors. It opens empty and fills when the server
	// answers OPEN_FAMILY_PEDIGREE. Only the player's own tree answers the
	// mouse. Esc closes it.
	class UIFamilyTree : public UIDragElement<PosFAMILYTREE>
	{
	public:
		static constexpr Type TYPE = UIElement::Type::FAMILYTREE;
		static constexpr bool FOCUSED = false;
		static constexpr bool TOGGLED = false;

		UIFamilyTree();

		void draw(float inter) const override;

		Cursor::State send_cursor(bool clicked, Point<int16_t> cursorpos) override;
		void remove_cursor() override;
		void send_key(int32_t keycode, bool pressed, bool escape) override;

		UIElement::Type get_type() const override;

		// FAMILY_CHART_RESULT came, already arranged.
		void set_chart(FamilyChart chart);
		// A relative logged in or out (FAMILY_NOTIFY_LOGIN_OR_LOGOUT).
		void set_online(const std::string& name, bool online);
		bool is_mine() const;

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		enum Buttons : uint16_t { JUNIOR, BYE };

		void lay_out();
		int16_t slot_at(Point<int16_t> local) const;
		bool hoverable(int16_t slot) const;
		std::string member_tooltip(size_t slot) const;
		void clear_tooltip();

		FamilyChart chart;

		Texture background, selected_glow;
		std::array<Texture, 2> leader_plate, other_plate;

		Text title, total, seniors;
		std::array<Text, FamilyChart::SLOTS> names, levels, empties, juniors;

		int16_t selected = -1;
		int16_t hovered = -1;
		bool tooltip = false;
	};
}
