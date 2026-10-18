#pragma once

#include "../UIElement.h"

#include "../Components/Textfield.h"

#include "../../Template/Optional.h"

#include <array>
#include <functional>
#include <string>
#include <vector>

namespace ms
{
	// The super megaphone's window (v83 FUN_00a0ece4 on UIWindow.img/Megaphone) and the triple megaphone's
	// (FUN_00a0ee22 on ArtMegaphone): one line or three, and whether listeners get an ear to whisper back by. It is
	// modal, and closes before the lines are checked.
	class UIMegaphone : public UIElement
	{
	public:
		static constexpr Type TYPE = UIElement::Type::MEGAPHONE;
		static constexpr bool FOCUSED = true;
		static constexpr bool TOGGLED = false;

		// `onsend` gets the lines as typed and the ear.
		UIMegaphone(bool triple, std::function<void(const std::vector<std::string> &lines, bool ear)> onsend);

		void draw(float alpha) const override;

		void update() override;

		void remove_cursor() override;

		Cursor::State send_cursor(bool clicked, Point<int16_t> cursorpos) override;

		void send_key(int32_t keycode, bool pressed, bool escape) override;

		UIElement::Type get_type() const override;

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		void send();

		void preview();

		void close();

		// Raises a notice, giving the keys back to the field that had them after it.
		void notice(const std::string &text);

		void ask(const std::string &question, std::function<void(UIMegaphone &window)> on_yes);

		// The open window after a notice, with field `refocus` focused again (none for -1).
		static Optional<UIMegaphone> after_notice(int16_t refocus);

		// Lets go of the keys for a notice. Returns the field that had them, or -1.
		int16_t release_field();

		std::vector<std::string> lines() const;

		enum Buttons : uint16_t
		{
			OK,
			CANCEL,
			EXAMPLE,
			PREVIEW
		};

		static constexpr size_t MAX_FIELDS = 3;

		bool triple;
		std::function<void(const std::vector<std::string> &lines, bool ear)> onsend;
		// Fields don't move once made: the UI keeps a pointer to the one with the keys.
		std::array<Textfield, MAX_FIELDS> fields;
		size_t field_count;
		std::array<Texture, 2> checkbox;
		Rectangle<int16_t> check_area;
		bool ear;
		bool held;
	};
}
