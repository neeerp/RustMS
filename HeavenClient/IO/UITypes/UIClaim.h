#pragma once

#include "../UIElement.h"

#include "../Components/Textfield.h"

#include "../../Character/Claim.h"
#include "../../Graphics/Texture.h"

#include <array>
#include <string>
#include <vector>

namespace ms
{
	// The first of v83's report windows, Claim/notice (0x7dac7b): what may
	// be reported, and whether it is a conversation or an illegal program.
	class UIClaimNotice : public UIElement
	{
	public:
		static constexpr Type TYPE = UIElement::Type::CLAIMNOTICE;
		static constexpr bool FOCUSED = true;
		static constexpr bool TOGGLED = false;

		// `rows`: the chat log's rows a report can quote, oldest first.
		UIClaimNotice(const std::string &name, const std::vector<std::string> &rows);

		void draw(float inter) const override;

		void send_key(int32_t keycode, bool pressed, bool escape) override;

		UIElement::Type get_type() const override;

		// CWvsContext::SendClaim (0xa2719c), from the status bar's report
		// button with no name or a character's menu with theirs: unless the
		// server takes no reports now, the notice opens.
		static void open(const std::string &name);

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		void choose(Claim::Kind kind);

		enum Buttons : uint16_t
		{
			CONVERSATION,
			PROGRAM,
			CANCEL
		};

		std::string name;
		std::vector<std::string> rows;
		Texture backgrnd;
	};

	// The second, Claim/backgrnd (0x7daf89): the character, the reason and
	// a description, sent by OK (0x7db8df).
	class UIClaim : public UIElement
	{
	public:
		static constexpr Type TYPE = UIElement::Type::CLAIM;
		static constexpr bool FOCUSED = true;
		static constexpr bool TOGGLED = false;

		UIClaim(Claim::Kind kind, const std::vector<std::string> &names, const std::vector<std::string> &rows);

		void draw(float inter) const override;

		void update() override;

		Cursor::State send_cursor(bool clicked, Point<int16_t> cursorpos) override;

		void send_key(int32_t keycode, bool pressed, bool escape) override;

		UIElement::Type get_type() const override;

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		void confirm();

		// An OK box over the window.
		void warn(const std::string &message);

		void close();

		void focus(size_t line);

		enum Buttons : uint16_t
		{
			OK,
			CANCEL,
			// Buttons draw in id order, and the character list opens over
			// the reason box.
			REASON,
			CHARACTER
		};

		Claim::Kind kind;
		std::vector<std::string> names;
		std::vector<std::string> rows;
		Texture backgrnd;
		std::array<Textfield, 4> lines;
	};
}
