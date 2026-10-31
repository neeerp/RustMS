#pragma once

#include "../UIElement.h"

#include "../Components/FocusRing.h"
#include "../Components/Slider.h"
#include "../Components/Textfield.h"

#include "../../Template/Optional.h"

#include <functional>
#include <string>
#include <vector>

namespace ms
{
	// The map list of a teleport rock (v83 FUN_008390d1, UIWindow.img/Teleport): choose a kept map or type a
	// character's name, keep the current map, or drop a kept one. It is modal, and closes before the rock is used.
	class UITeleportRock : public UIElement
	{
	public:
		static constexpr Type TYPE = UIElement::Type::TELEPORTROCK;
		static constexpr bool FOCUSED = true;
		static constexpr bool TOGGLED = false;

		// `onmove` gets the typed name, or an empty name and the chosen map.
		UITeleportRock(bool vip, std::function<void(const std::string &name, int32_t map_id)> onmove);

		void draw(float alpha) const override;

		void update() override;

		void remove_cursor() override;

		Cursor::State send_cursor(bool clicked, Point<int16_t> cursorpos) override;

		void send_key(int32_t keycode, bool pressed, bool escape) override;

		bool send_focus_key(int32_t keycode, bool pressed, bool shift) override;

		UIElement::Type get_type() const override;

		// Shows the kept maps again, keeping the selection and the scroll.
		void refresh();

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		// Selects the row under `cursor`, or deselects from the area below the list (FUN_00839dc0).
		bool select_at(Point<int16_t> cursor);

		void keep();

		void drop();

		void move();

		void close();

		// Raises a notice, giving the field's keys back after it.
		void notice(const std::string &text);

		void ask(const std::string &question, std::function<void(UITeleportRock &window)> on_yes);

		// The open window after a notice, with its field focused again if it was before.
		static Optional<UITeleportRock> after_notice(bool refocus);

		// Lets go of the keys for a notice. Returns whether the field had them.
		bool release_field();

		enum Buttons : uint16_t
		{
			REGISTER,
			DELETE,
			MOVE,
			CLOSE
		};

		struct Row
		{
			Text normal;
			Text selected;
		};

		bool vip;
		std::function<void(const std::string &name, int32_t map_id)> onmove;
		std::vector<int32_t> maps;
		std::vector<Row> rows;
		Texture select;
		Text counter;
		int16_t counter_width;
		Textfield namefield;
		Slider slider;
		// Register, Delete, Move, Close and the field (FUN_008392c1); the VIP list's scrollbar is left out.
		FocusRing ring;
		int16_t first;
		int16_t selected;
		bool held;
	};
}
