#pragma once

#include "../UIElement.h"

#include "../Components/Textfield.h"

#include "../../Template/Optional.h"

#include <nlnx/node.hpp>

#include <array>
#include <functional>
#include <optional>
#include <string>
#include <vector>

namespace ms
{
	// The super megaphone's window (v83 FUN_00a0ece4 on UIWindow.img/Megaphone), the triple megaphone's
	// (FUN_00a0ee22 on ArtMegaphone) and the avatar megaphone's (FUN_007f1196 on AvatarMegaphone): one line, three,
	// or one box that wraps onto four, and whether listeners get an ear to whisper back by. It is modal, and closes
	// before the lines are checked. The triple megaphone's Special Character button opens the item's samples over it
	// (FUN_00999320).
	class UIMegaphone : public UIElement
	{
	public:
		static constexpr Type TYPE = UIElement::Type::MEGAPHONE;
		static constexpr bool FOCUSED = true;
		static constexpr bool TOGGLED = false;

		// For megaphone `item_id`. `onsend` gets the lines as typed and the ear.
		UIMegaphone(int32_t item_id, std::function<void(const std::vector<std::string> &lines, bool ear)> onsend);

		void draw(float alpha) const override;

		void update() override;

		void remove_cursor() override;

		Cursor::State send_cursor(bool clicked, Point<int16_t> cursorpos) override;

		void send_key(int32_t keycode, bool pressed, bool escape) override;

		UIElement::Type get_type() const override;

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

		bool button_covered(uint16_t buttonid, Point<int16_t> cursorpos) const override;

	private:
		void send();

		void open_samples();

		// Closes the samples, putting sample `index` in the lines if one was picked.
		void use_sample(std::optional<uint16_t> index);

		Cursor::State send_samples_cursor(bool press, bool release, bool clicked, Point<int16_t> cursorpos);

		// Gives the keys to field `index`, if there is one.
		void focus_field(int16_t index);

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
			PREVIEW,
			// The samples follow, one button each.
			SAMPLE
		};

		static constexpr size_t MAX_FIELDS = 3;

		bool triple;
		bool avatar;
		// Item.nx's Cash entry for the megaphone.
		nl::node item;
		std::function<void(const std::vector<std::string> &lines, bool ear)> onsend;
		// Fields don't move once made: the UI keeps a pointer to the one with the keys.
		std::array<Textfield, MAX_FIELDS> fields;
		size_t field_count;
		std::array<Texture, 2> checkbox;
		Rectangle<int16_t> check_area;
		bool ear;
		bool held;
		// The checkbox was pressed, and toggles when the button comes up.
		bool checking;
		// The samples' popup: whether it is open, where it is, and its art.
		bool sampling;
		Point<int16_t> samples_pos;
		uint16_t sample_count;
		Texture samples_top;
		Texture samples_row;
		Texture samples_bottom;
	};
}
